// specification: https://rdb.fnordig.de/file_format.html

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};

use crate::db::DB;

#[derive(Clone)]
pub struct RDB {
    db: DB,
}

impl RDB {
    pub fn new(db: DB) -> RDB {
        let rdb = RDB { db };
        tokio::spawn(handle_saving(rdb.clone()));
        rdb
    }
    fn save(&self) {
        let store = self.db.db.state.lock().unwrap();
        let writer = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("rdb.rdb")
            .unwrap();
        let mut writer = BufWriter::new(writer);
        // Magic string
        writer.write_all("REDIS".as_bytes()).unwrap();
        // RDB Version
        writer.write_all("0003".as_bytes()).unwrap();
        // Aux fields - Metadata
        writer.write_all(&[0xFA]).unwrap();
        // Created At
        self.write_string_encoded(&mut writer, "created_at");
        self.write_string_encoded(&mut writer, chrono::Utc::now().to_string().as_str());
        // Database selection section
        writer.write_all(&[0xFE]).unwrap();
        self.write_string_encoded(&mut writer, "00");
        writer.write_all(&[0xFB]).unwrap();
        // Size of entries
        self.write_integer_encoded(&mut writer, store.entries.len());
        // Size of ttls
        self.write_integer_encoded(&mut writer, store.ttls.len());

        // Key value pairs
        // Map through all entries
        for (key, value) in store.entries.iter() {
            match value.expires_at {
                Some(ttl) => self.write_key_value_ttl_sec(
                    &mut writer,
                    key,
                    &String::from_utf8_lossy(&value.value).clone(),
                    (ttl - Instant::now()).as_secs() as usize,
                ),
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
    pub async fn load(&self) {}

    fn write_string_encoded(&self, writer: &mut BufWriter<File>, input: &str) {
        self.write_integer_encoded(writer, input.len());
        writer.write_all(input.as_bytes()).unwrap()
    }

    fn write_key_value_ttl_sec(
        &self,
        writer: &mut BufWriter<File>,
        key: &str,
        value: &str,
        ttl: usize,
    ) {
        writer.write_all(&[0xFD]).unwrap();

        writer.write_all(&(ttl as u32).to_be_bytes()).unwrap(); // Replace with ttl
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
        let length = if input < 64 {
            let length = input as u8;

            let bits = 0b0000_0000_u8;

            bits | length
        } else {
            todo!()
        };

        writer.write_all(&[length]).unwrap();
    }
}

async fn handle_saving(rdb: RDB) {
    loop {
        // Will save every minute
        tokio::time::sleep(Duration::from_secs(10)).await;
        rdb.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test() {
        let mut db = DB::new();
        db.set("key".to_string(), "value".as_bytes().to_vec(), None);
        db.set("key2".to_string(), "value".as_bytes().to_vec(), Some(2));
        let rdb = RDB::new(db);
        rdb.save();
    }
}
