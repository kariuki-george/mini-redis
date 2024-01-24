use std::collections::VecDeque;

use mini_redis::{connection::Connection, frame::Frame};

use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    set().await;
    get().await;
}

async fn set() {
    let stream = TcpStream::connect("localhost:6379").await.unwrap();

    let mut client = Connection::new(stream);

    // Read data from the stream

    let input = VecDeque::from([
        Frame::SimpleString("set".to_string()),
        Frame::SimpleString("user".to_string()),
        Frame::SimpleString("kariuki".to_string()),
    ]);

    //  let input = String::from("ping");

    match client.write_all(Frame::Array(input)).await {
        Ok(_) => {
            let frame = client.read_frame().await;
            println!("{frame:?}");

            // handle_frame(frame)
        }
        Err(e) => println!("{e:?}"),
    }

    client.shutdown().await;
}

async fn get() {
    let stream = TcpStream::connect("localhost:6379").await.unwrap();

    let mut client = Connection::new(stream);

    // Read data from the stream

    let input = VecDeque::from([
        Frame::SimpleString("get".to_string()),
        Frame::SimpleString("user".to_string()),
    ]);

    //  let input = String::from("ping");

    match client.write_all(Frame::Array(input)).await {
        Ok(_) => {
            let frame = client.read_frame().await;
            println!("{frame:?}");

            // handle_frame(frame)
        }
        Err(e) => println!("{e:?}"),
    }

    client.shutdown().await;
}
