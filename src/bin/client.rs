use mini_redis::{connection::Connection, frame::Frame};

use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    let stream = TcpStream::connect("localhost:6379").await.unwrap();
    let mut client = Connection::new(stream);

    // Read data from the stream

    match client.write(Frame::String("input".to_string())).await {
        Ok(_) => {
            let frame: Result<Option<Frame>, mini_redis::connection::ConnectionError> =
                client.read_frame().await;
            handle_frame(frame)
        }
        Err(e) => println!("{e:?}"),
    }
    // //thread::sleep(Duration::from_secs(20));
}

fn handle_frame(frame: Result<Option<Frame>, mini_redis::connection::ConnectionError>) {
    match frame {
        Ok(opt) => println!("{opt:?}"),
        Err(e) => match e {
            mini_redis::connection::ConnectionError::FrameError(_) => println!("{e:?}"),
            mini_redis::connection::ConnectionError::IOError(e) => {
                println!("{e:?}")
            }
        },
    }
}
