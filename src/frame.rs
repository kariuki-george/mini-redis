use std::io::Cursor;

use bytes::{Buf, Bytes};

#[derive(Debug, PartialEq)]
pub enum Frame {
    String(String),
    Error(String),
    Integer(usize),
    Null,
    Booleans(char),
    Array(Vec<Frame>),
    Bulk(Bytes),
}

#[derive(Debug)]
pub enum FrameError {
    Incomplete,
    Other(String),
}

impl Frame {
    // A redis string is parsed into an array of valid frames
    // Example -> An array *2\r\n$5\r\get\r\n$5\r\key\r\n
    // Output ->  [String(get), String(key)]

    // Checks if a complete frame can be deserialized from a buffer.
    // Optimized to be fast thus not allocations.

    pub fn check(cursor: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        match get_first_byte(cursor)? {
            b'+' => {
                //    This is a simple String
                // Parse the contents

                get_simple_string(cursor)?;

                Ok(())
            }
            _ => Err(FrameError::Other(String::from(
                "Protocol Error: Invalid input ",
            ))),
        }
    }

    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
        match get_first_byte(cursor)? {
            b'+' => {
                //    This is a simple String
                // Parse the contents
                let bytes = get_simple_string(cursor)?;
                let string = String::from_utf8_lossy(bytes).to_string();
                Ok(Frame::String(string))
            }
            b'-' => {
                //    This is a simple Error
                // Parse the contents
                let bytes = get_simple_string(cursor)?;
                let error = String::from_utf8_lossy(bytes).to_string();
                Ok(Frame::Error(error))
            }
            b':' => {
                //    This is an Integer
                // Parse the contents
                let integer = get_integer(cursor)?;

                Ok(Frame::Integer(integer))
            }
            _ => Err(FrameError::Other(String::from(
                "Protocol Error: Invalid input ",
            ))),
        }
    }
}

fn get_first_byte(cursor: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !cursor.has_remaining() {
        return Err(FrameError::Incomplete);
    }

    Ok(cursor.get_u8())
}

fn get_integer(cursor: &mut Cursor<&[u8]>) -> Result<usize, FrameError> {
    let start = cursor.position() as usize;
    let end = cursor.get_ref().len() - 1;

    for position in start..end {
        if cursor.get_ref()[position] == b'\r' && cursor.get_ref()[position + 1] == b'\n' {
            cursor.set_position((position + 2) as u64);
            let bytes = &cursor.get_ref()[start..position];
            return atoi::atoi::<usize>(bytes)
                .ok_or_else(|| FrameError::Other("Protocol Error: Invalid input".to_string()));
        }
    }
    Err(FrameError::Incomplete)
}

fn get_simple_string<'a>(cursor: &'a mut Cursor<&[u8]>) -> Result<&'a [u8], FrameError> {
    let start = cursor.position() as usize;
    let end = cursor.get_ref().len() - 1;

    for position in start..end {
        if cursor.get_ref()[position] == b'\r' && cursor.get_ref()[position + 1] == b'\n' {
            cursor.set_position((position + 2) as u64);
            return Ok(&cursor.get_ref()[start..position]);
        }
    }
    Err(FrameError::Incomplete)
}
