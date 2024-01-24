use std::collections::VecDeque;

use crate::{db::DB, frame::Frame};

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
        if frames.len() > 2 {
            return Err(RunnerError::Incomplete);
        }

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

        self.db.set(key, value.as_bytes().to_owned(), None);
        Ok(Frame::SimpleString("OK".to_string()))
    }

    fn run_get(&mut self, frames: &mut VecDeque<Frame>) -> Result<Frame, RunnerError> {
        if frames.len() > 1 {
            return Err(RunnerError::Incomplete);
        }

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
