// specification: https://rdb.fnordig.de/file_format.html

use std::fs::{self, File};
use std::io::{BufWriter, Cursor, Read, Write};

use crate::db::DB;

#[derive(Clone)]
pub struct RDB {
    db: DB,
}

impl RDB {
    pub fn new(db: DB) -> RDB {
        tracing::info!("RDB: Starting RDB storage service");
        let rdb = RDB { db };
        tokio::spawn(handle_saving(rdb.clone()));
        rdb
    }
    fn save(&self) {
        tracing::info!("RDB: Flushing DB into RDB Storage");
        let file = std::env::var("RDB_URL");
        let file = match file {
            Ok(file) => file,
            Err(_) => {
                tracing::warn!("RDB: Could not find RDB_URL env var, defaulting to rdb.rdb");
                "rdb.rdb".to_string()
            }
        };

        let store = self.db.db.state.lock().unwrap();
        let writer = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file)
            .unwrap();
        let mut writer = BufWriter::new(writer);
        // Magic string
        writer.write_all("REDIS".as_bytes()).unwrap();
        // RDB Version as 4 bytes
        writer.write_all("0003".as_bytes()).unwrap();
        // Aux fields - Metadata
        writer.write_all(&[0xFA]).unwrap();
        // Created At
        self.write_string_encoded(&mut writer, "ctime");
        self.write_string_encoded(&mut writer, chrono::Utc::now().to_string().as_str());
        // Database selection section
        writer.write_all(&[0xFE]).unwrap();
        self.write_string_encoded(&mut writer, "0");
        writer.write_all(&[0xFB]).unwrap();
        // Size of entries
        self.write_integer_encoded(&mut writer, store.entries.len());
        // Size of ttls
        self.write_integer_encoded(&mut writer, store.ttls.len());

        // Key value pairs
        // Map through all entries
        for (key, value) in store.entries.iter() {
            match value.expires_at {
                Some(ttl) => {
                    self.write_key_value_ttl_sec(
                        &mut writer,
                        key,
                        &String::from_utf8_lossy(&value.value).clone(),
                        ttl,
                    );
                }
                None => self.write_key_value_no_ttl(
                    &mut writer,
                    key,
                    &String::from_utf8_lossy(&value.value).clone(),
                ),
            }
        }

        // End of file
        writer.write_all(&[0xFF]).unwrap();

        // 8-byte checksum
        // TODO
    }
    pub async fn load(&mut self) -> Result<(), String> {
        let file = std::env::var("RDB_URL");
        let file = match file {
            Ok(file) => file,
            Err(_) => {
                tracing::warn!("RDB: Could not find RDB_URL env var, skipping RDB data reloading");
                return Ok(());
            }
        };

        let reader = fs::OpenOptions::new().read(true).open(file);
        let mut reader = match reader {
            Ok(file) => file,
            Err(_) => {
                tracing::warn!("RDB file not detected correctly. Continuing without loading it");
                return Ok(());
            }
        };

        // Read the whole file at one go
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer).unwrap();

        let mut cursor = Cursor::new(&buffer);

        // Check the magic string
        let mut magic_string_buf = vec![0; 5];
        cursor.read_exact(&mut magic_string_buf).unwrap();

        if String::from_utf8_lossy(&magic_string_buf) != "REDIS" {
            tracing::error!("RDB: Invalid rdb file passed");
            return Err("Invalid rdb file passed".to_string());
        }
        tracing::info!("RDB: Attempting to restore state from RDB file");

        // Check RDB VERSION
        let mut version = vec![0; 4];
        cursor.read_exact(&mut version).unwrap();

        let version = String::from_utf8_lossy(&version);
        tracing::info!("RDB: Version: {version:?}");

        loop {
            // Get the next section from the next byte
            let position = cursor.position();
            let byte = self.get_next_byte(&mut cursor).unwrap();

            match byte {
                0xFA => loop {
                    self.get_aux_field_values(&mut cursor).unwrap();
                    let position = cursor.position();
                    let byte = self.get_next_byte(&mut cursor).unwrap();
                    if byte == 0xFE {
                        cursor.set_position(position);
                        break;
                    }
                },
                0xFE => {
                    let database_number = self.read_string_encoded(&mut cursor).unwrap();
                    tracing::info!("RDB: Attempting to restore DB: {database_number:?}");
                }
                0xFB => {
                    let hashtable_length = self.read_integer_encoded(&mut cursor).unwrap();
                    let ttls_length = self.read_integer_encoded(&mut cursor).unwrap();
                    tracing::info!(
                        "RDB: Trying to restore {hashtable_length} KV, {ttls_length} ttls"
                    );
                }
                0xFD => {
                    let mut ttl_buffer: [u8; 4] = [0; 4];
                    cursor.read_exact(&mut ttl_buffer).unwrap();
                    let ttl = u32::from_le_bytes(ttl_buffer);

                    let _value_type = self.read_string_encoded(&mut cursor).unwrap();

                    let key = self.read_string_encoded(&mut cursor).unwrap();

                    let value = self.read_string_encoded(&mut cursor).unwrap();

                    if ttl < chrono::Utc::now().timestamp() as u32 {
                        // Drop that key value pair as per rdb protocol
                        continue;
                    }

                    self.db.set(
                        key,
                        value.as_bytes().to_vec(),
                        Some(ttl - chrono::Utc::now().timestamp() as u32),
                    );
                }

                0xFF => {
                    tracing::info!("RDB: END of RDB: Restored data successfully");

                    return Ok(());
                }
                _value => {
                    cursor.set_position(position);
                    // Key-Value without expiry
                    let value_type = self.read_string_encoded(&mut cursor).unwrap();

                    if value_type != "0" {
                        tracing::error!("RDB: Unsupported Value Encoding found");
                        return Err("Unsupported Value Encoding found".to_string());
                    }

                    let key = self.read_string_encoded(&mut cursor).unwrap();

                    let value = self.read_string_encoded(&mut cursor).unwrap();
                    self.db.set(key, value.as_bytes().to_vec(), None);
                }
            }
        }
    }

    fn get_next_byte(&self, cursor: &mut Cursor<&Vec<u8>>) -> std::io::Result<u8> {
        let mut byte = vec![0; 1];
        cursor.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    fn get_aux_field_values(&self, cursor: &mut Cursor<&Vec<u8>>) -> std::io::Result<()> {
        let key = self.read_string_encoded(cursor)?;
        let value = &self.read_string_encoded(cursor)?;
        tracing::info!("RDB: METADATA: {key} {value}");

        Ok(())
    }

    fn read_string_encoded(&self, cursor: &mut Cursor<&Vec<u8>>) -> std::io::Result<String> {
        let length = self.read_integer_encoded(cursor)?;

        let mut buffer = vec![0; length];

        cursor.read_exact(&mut buffer)?;

        return Ok(String::from_utf8_lossy(&buffer).to_string());
    }

    fn read_integer_encoded(&self, cursor: &mut Cursor<&Vec<u8>>) -> std::io::Result<usize> {
        let byte = self.get_next_byte(cursor)?;

        // 01
        // let input = input as u16;

        // let byte_one = 0b0100_0000 | ((input >> 8) & 0b0011_1111) as u8;
        // let byte_two = input as u8;

        // writer.write_all(&[byte_one, byte_two]).unwrap();

        let significant_bits = (byte >> 6) & 0b11;

        if significant_bits == 0b00 {
            let integer = byte & 0b0011_1111;
            return Ok(integer as usize);
        } else if significant_bits == 0b01 {
            let byte = byte & 0b0011_1111;
            let byte2 = self.get_next_byte(cursor)?;

            let integer = ((byte as usize) << 8) | byte2 as usize;

            return Ok(integer);
        }

        todo!()
    }

    fn write_string_encoded(&self, writer: &mut BufWriter<File>, input: &str) {
        self.write_integer_encoded(writer, input.len());
        writer.write_all(input.as_bytes()).unwrap()
    }

    fn write_key_value_ttl_sec(
        &self,
        writer: &mut BufWriter<File>,
        key: &str,
        value: &str,
        ttl: u32,
    ) {
        writer.write_all(&[0xFD]).unwrap();

        writer.write_all(&(ttl).to_le_bytes()).unwrap(); // Replace with ttl
                                                         // Limit the type of value to only strings
        self.write_string_encoded(writer, "0");
        self.write_string_encoded(writer, key);
        self.write_string_encoded(writer, value)
    }
    fn write_key_value_no_ttl(&self, writer: &mut BufWriter<File>, key: &str, value: &str) {
        self.write_string_encoded(writer, "0");
        self.write_string_encoded(writer, key);
        self.write_string_encoded(writer, value)
    }
    fn write_integer_encoded(&self, writer: &mut BufWriter<File>, input: usize) {
        if input < 64 {
            let length = input as u8;

            let bits = 0b0000_0000_u8;

            writer.write_all(&[bits | length]).unwrap();
        } else if input <= 16383 {
            // 01
            let input = input as u16;

            let byte_one = 0b0100_0000 | ((input >> 8) & 0b0011_1111) as u8;
            let byte_two = input as u8;

            writer.write_all(&[byte_one, byte_two]).unwrap();
        } else if input <= 4294967295 {
            // 2^32-1
            todo!()
        } else {
            // Special formatting
            // 11
            // Out of scope as we are storing strings only
            todo!()
        };
    }
}

async fn handle_saving(rdb: RDB) {
    tracing::info!("RDB: Starting RDB storage background worker");

    let flush_every = std::env::var("FLUSH_EVERY");

    let flush_every = match flush_every {
        Ok(time) => {
            match time.parse::<u64>() {
                Ok(time) => {
                    if time > 68719476734 {
                        // Tokio sleep max limit
                        tracing::error!("Max number of seconds supported is 68719476734, defaulting to 68719476734");
                        68719476734
                    } else {
                        time
                    }
                }
                Err(_) => {
                    tracing::error!(
                        "Invalid FLUSH_EVERY env var provided, defaulting to 60 seconds"
                    );
                    60
                }
            }
        }
        Err(_) => {
            tracing::warn!("Failed to read FLUSH_EVERY env var, defaulting to 60 seconds");
            60
        }
    };

    loop {
        // Will save every 30 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(flush_every as u64)).await;
        rdb.save()
    }
}
