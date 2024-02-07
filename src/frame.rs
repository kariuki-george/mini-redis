use std::{collections::VecDeque, io::Cursor};

use bytes::Buf;

/*
A redis string is parsed into a  valid frame
Example ->
    redis string input -> "*2\r\n$5\r\get\r\n$5\r\key\r\n:
    Output ->  [String(get), String(key)]
*/

#[derive(Debug, PartialEq, Clone)]
pub enum Frame {
    SimpleString(String),
    SimpleError(String),
    Integer(usize),
    Array(VecDeque<Frame>),
}

#[derive(Debug)]
pub enum FrameError {
    Incomplete,
    Other(String),
}

impl Frame {
    // Checks if a complete frame can be deserialized from a buffer.
    // Optimized to be fast thus not allocations.
    pub fn check(cursor: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        match get_first_byte(cursor)? {
            b'+' => {
                //    This is a simple String

                get_simple_string(cursor)?;

                Ok(())
            }
            b'-' => {
                //    This is a simple String

                get_simple_string(cursor)?;

                Ok(())
            }
            b':' => {
                //   Check an integer

                get_integer(cursor)?;

                Ok(())
            }
            b'*' => check_array(cursor),
            _ => Err(FrameError::Other(String::from(
                "Protocol Error: Invalid input ",
            ))),
        }
    }

    // Deserializes a frame from a buffer

    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
        match get_first_byte(cursor)? {
            b'+' => {
                //    This is a simple String
                // Parse the contents
                let bytes = get_simple_string(cursor)?;
                let string = String::from_utf8_lossy(bytes).to_string();

                Ok(Frame::SimpleString(string))
            }
            b'-' => {
                //    This is a simple Error
                // Parse the contents
                let bytes = get_simple_string(cursor)?;
                let error = String::from_utf8_lossy(bytes).to_string();
                Ok(Frame::SimpleError(error))
            }
            b':' => {
                //   Check an integer

                let integer = get_integer(cursor)?;

                Ok(Frame::Integer(integer))
            }

            b'*' => get_array(cursor),

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

fn check_array(cursor: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
    // First element is the number of items
    // The rest are individual frames

    let num_of_items = get_integer(cursor)?;

    for _ in 0..num_of_items {
        Frame::check(cursor)?;
    }

    Ok(())
}

fn get_array(cursor: &mut Cursor<&[u8]>) -> Result<Frame, FrameError> {
    // First element is the number of items
    // The rest are individual frames

    let num_of_items = get_integer(cursor)?;

    let mut frames = VecDeque::new();

    for _ in 0..num_of_items {
        let frame = Frame::parse(cursor)?;
        frames.push_back(frame);
    }

    Ok(Frame::Array(frames))
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
