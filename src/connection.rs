use crate::frame::{Frame, FrameError};
use bytes::{Buf, BytesMut};
use std::io::{Cursor, ErrorKind};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

#[derive(Debug)]
pub enum ConnectionError {
    FrameError(FrameError),
    IOError(ErrorKind),
}

/**
 * Handles all tcp communications.
 * It takes in a tcp stream and allocated buffer to read data from it.
 * It reads data from the tcpstream buffer into it's buffer thus returning a frame if successful.
 * It writes frames into the tcpstream and flushes it when done ensuring it reaches the client.
 */
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    /**
     * Takes in a tcpstream to be used for reading and writing frames.
     * Pre-allocates a buffer in this case 2KB. It can be adjusted according to stats.
     * This allows us not to preallocate alot of buffer that ends up getting unused.
     */
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            buffer: BytesMut::with_capacity(2 * 1024),
            stream: BufWriter::new(stream),
        }
    }

    /**
     * Tries to read a frame from the tcpStream.
     * Returns a frame if one is found else None if no frame is in the stream.
     *
     */

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, ConnectionError> {
        loop {
            // Try to read a parse a frame to completion.

            if let Some(frame) = self.parse_frame().map_err(ConnectionError::FrameError)? {
                return Ok(Some(frame));
            }

            //Else, if incomplete, try to read more data from the stream

            if 0 == self
                .stream
                .read_buf(&mut self.buffer)
                .await
                .map_err(|_| ConnectionError::IOError(ErrorKind::InvalidInput))?
            {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    // The client possibly closed the connection or some networking issue occurred
                    return Err(ConnectionError::IOError(ErrorKind::ConnectionReset));
                }
            }
        }
    }

    /**
     * Use a cursor to read a frame from the stream.
     * A frame can be parsed from multiple buffers from the same stream, one after the other
     */
    fn parse_frame(&mut self) -> Result<Option<Frame>, FrameError> {
        let mut cursor: Cursor<&[u8]> = Cursor::new(&self.buffer[..]);
        match Frame::check(&mut cursor) {
            Ok(_) => {
                let len = cursor.position();
                cursor.set_position(0);
                let frame = Frame::parse(&mut cursor)?;
                self.buffer.advance(len as usize);

                Ok(Some(frame))
            }

            Err(err) => match err {
                // Read more data from the stream
                FrameError::Incomplete => Ok(None),
                FrameError::Other(input) => Err(FrameError::Other(input)),
            },
        }
    }
    /**
     * Writes frames into the tcpstream thus responding to the client.
     *
     */
    pub async fn write_all(&mut self, frame: Frame) -> std::io::Result<()> {
        match frame {
            Frame::SimpleString(_) => {
                self.write(frame).await?;
            }
            Frame::SimpleError(_) => {
                self.write(frame).await?;
            }
            Frame::Integer(_) => {
                self.write(frame).await?;
            }

            Frame::Array(frames) => {
                self.stream.write_all('*'.to_string().as_bytes()).await?;
                self.stream
                    .write_all(format!("{}\r\n", frames.len()).as_bytes())
                    .await?;

                for frame in frames {
                    self.write(frame.to_owned()).await?;
                }
            }
        }

        self.stream.flush().await?;

        Ok(())
    }

    async fn write(&mut self, frame: Frame) -> std::io::Result<()> {
        match frame {
            Frame::SimpleString(input) => {
                let mut data = String::new();
                data.push('+');
                data.push_str(input.as_str());
                data.push_str("\r\n");

                self.stream.write_all(data.as_bytes()).await?;
            }
            Frame::Integer(input) => {
                // 🦥
                let mut data = String::new();
                data.push(':');
                data.push_str(input.to_string().as_str());
                data.push_str("\r\n");

                self.stream.write_all(data.as_bytes()).await?;
            }
            Frame::SimpleError(input) => {
                let mut data = String::new();
                data.push('-');
                data.push_str(input.as_str());
                data.push_str("\r\n");

                self.stream.write_all(data.as_bytes()).await?;
            }
            // TODO: Handle these arms
            Frame::Array(_) => {}
        }

        Ok(())
    }
    pub async fn shutdown(&mut self) {
        let _ = self.stream.shutdown().await;
    }
}
