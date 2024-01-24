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
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        // The buffer will store atmost 2 KBs.
        Connection {
            buffer: BytesMut::with_capacity(2 * 1024),
            stream: BufWriter::new(stream),
        }
    }

    /**
     * As a result of a static buffer size,
     * We might need to read the stream a couple times to read a full/valid frame.
     * This results into several states
     * 1. A frame is read from the buffer
     * 2. Buffer is empty thus no frame
     * 3. Frame requires more data than the buffer can handle thus another loop to read more data.
     * 4. Stream is empty, buffer is empty, but a frame is incomplete thus connection reset error
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
                    return Err(ConnectionError::IOError(ErrorKind::ConnectionReset));
                }
            }
        }
    }

    /**
     * Use a cursor to read a frame from the stream.
     * A frame can be parsed from multiple buffers from the same stream, one after the other
     *      *
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

    pub async fn write_all(&mut self, frame: Frame) -> std::io::Result<()> {
        match frame {
            Frame::SimpleString(_) => {
                self.write(frame).await?;
            }
            Frame::SimpleError(_) => {
                self.write(frame).await?;
            }
            Frame::Integer(_) => {}
            Frame::Null => {}
            Frame::Booleans(_) => {}
            Frame::Array(frames) => {
                self.stream.write_all('*'.to_string().as_bytes()).await?;
                self.stream
                    .write_all(format!("{}\r\n", frames.len()).as_bytes())
                    .await?;

                for frame in frames {
                    self.write(frame.to_owned()).await?;
                }
            }
            Frame::Bulk(_) => {}
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
            Frame::SimpleError(input) => {
                let mut data = String::new();
                data.push('-');
                data.push_str(input.as_str());
                data.push_str("\r\n");

                self.stream.write_all(data.as_bytes()).await?;
            }
            Frame::Integer(_) => {}
            Frame::Null => {}
            Frame::Booleans(_) => {}
            Frame::Bulk(_) => {}
            Frame::Array(_) => {}
        }

        Ok(())
    }
    pub async fn shutdown(&mut self) {
        let _ = self.stream.shutdown().await;
    }
}
