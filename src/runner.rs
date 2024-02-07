use std::collections::VecDeque;

use crate::{db::DB, frame::Frame};

/**
 * Handles execution of a frame.
 * It parses commands from a frame and executes them against the db if necessary
 */
pub struct Runner<'a> {
    db: &'a mut DB,
}

#[derive(Debug)]
pub enum RunnerError {
    Incomplete,
    Other(String),
    Unsupported,
}

impl<'a> Runner<'a> {
    pub fn new(db: &mut DB) -> Runner {
        Runner { db }
    }
    pub fn run(&mut self, frame: Frame) -> Result<Frame, RunnerError> {
        match frame {
            Frame::SimpleString(input) => self.run_string(input.to_owned()),
            Frame::Array(mut input) => self.run_array(&mut input),
            _ => Err(RunnerError::Unsupported),
        }
    }

    fn run_array(&mut self, frames: &mut VecDeque<Frame>) -> Result<Frame, RunnerError> {
        let command = frames.pop_front().ok_or(RunnerError::Incomplete)?;
        match command {
            Frame::SimpleString(input) => self.run_cmd(input, frames),
            _ => Err(RunnerError::Unsupported),
        }
    }
    fn run_cmd(
        &mut self,
        input: String,
        frames: &mut VecDeque<Frame>,
    ) -> Result<Frame, RunnerError> {
        match input.to_uppercase().as_str() {
            "SET" => self.run_set(frames),
            "GET" => self.run_get(frames),
            _ => Err(RunnerError::Unsupported),
        }
    }
    fn run_set(&mut self, frames: &mut VecDeque<Frame>) -> Result<Frame, RunnerError> {
      
        let key = frames.pop_front().ok_or(RunnerError::Incomplete)?;

        let key = match key {
            Frame::SimpleString(input) => input,
            _ => return Err(RunnerError::Unsupported),
        };

        let value = frames.pop_front().ok_or(RunnerError::Incomplete)?;

        let value = match value {
            Frame::SimpleString(input) => input,
            _ => return Err(RunnerError::Unsupported),
        };

        // Check if ttl options is in the next frame
        // If available -  parse the value
        // Ignore the next frame

        let ttl = match frames.front() {
            Some(frame) => {
                if let Frame::SimpleString(input) = frame {
                    match input.to_uppercase().as_str() {
                        "EX" => {
                            frames.pop_front();
                            let ttl_frame = frames.pop_front().ok_or(RunnerError::Incomplete)?;
                            match ttl_frame {
                                Frame::Integer(ttl) => Some(ttl),
                                _ => {
                                    return Err(RunnerError::Other(
                                        "Incorrect ttl data type. Should be an integer".to_string(),
                                    ))
                                }
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            None => None,
        };

        self.db.set(key, value.as_bytes().to_owned(), ttl);
        Ok(Frame::SimpleString("OK".to_string()))
    }

    fn run_get(&mut self, frames: &mut VecDeque<Frame>) -> Result<Frame, RunnerError> {
       

        let key = frames.pop_front().ok_or(RunnerError::Incomplete)?;

        let key = match key {
            Frame::SimpleString(input) => input,
            _ => return Err(RunnerError::Unsupported),
        };

        let value = self.db.get(&key);
        match value {
            Some(value) => Ok(Frame::SimpleString(
                String::from_utf8_lossy(&value).into_owned(),
            )),
            None => Ok(Frame::SimpleError("Nill".to_string())),
        }
    }

    fn run_string(&self, input: String) -> Result<Frame, RunnerError> {
        match input.to_uppercase().as_str() {
            "PING" => Ok(Frame::SimpleString("PONG".to_string())),
            _ => Err(RunnerError::Unsupported),
        }
    }
}
