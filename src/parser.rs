// use std::{
//     collections::VecDeque,
//     io::{BufWriter, Cursor},
//     net::TcpStream,
// };

// use crate::frame::Frame;

// struct Parser {
//     frames: VecDeque<Frame>,
//     add_more: bool,
// }

// enum Value {
//     STRING(String),
//     INTEGER(usize),
// }
// enum Commands {
//     SET(String, Value, Option<usize>),
//     GET(String),
//     EXPIRE(String, usize),
// }

// impl Parser {
//     fn new(&mut self) -> Parser {
//         Parser {
//             frames: VecDeque::new(),
//             add_more: true,
//         }
//     }
//     fn push(&mut self, frame: Frame) {
//         self.frames.push_back(frame)
//     }

//     fn pop(&mut self) -> Option<Frame> {
//         self.frames.pop_front()
//     }
// }
