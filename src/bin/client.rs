use std::{collections::VecDeque, ops::Sub};

use chrono::Timelike;
use mini_redis::{connection::Connection, frame::Frame};

use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to read .env file");
    set().await;
    let start = chrono::Utc::now();
    for _ in 0..10000 {
        set().await;
        get().await;
    }
    let stop = chrono::Utc::now();
    println!(
        "Took {} seconds to make full 20000 (SET->GET) tcp requests",
        stop.sub(start).to_std().unwrap().as_secs()
    );
}

async fn set() {
    let addr = std::env::var("ADDR").expect("ADDR env var not provided");

    let stream = TcpStream::connect(addr).await.unwrap();

    let mut client = Connection::new(stream);

    // Read data from the stream
    let key = format!("user-{}", chrono::Utc::now().second());

    let input = VecDeque::from([
        Frame::SimpleString("set".to_string()),
        Frame::SimpleString(key),
        Frame::SimpleString("kariuki".to_string()),
        Frame::SimpleString("EX".to_string()),
        Frame::Integer(40),
    ]);

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
    let addr = std::env::var("ADDR").expect("ADDR env var not provided");

    let stream = TcpStream::connect(addr).await.unwrap();

    let mut client = Connection::new(stream);

    // Read data from the stream
    let key = format!("user-{}", chrono::Utc::now().second());

    let input = VecDeque::from([
        Frame::SimpleString("get".to_string()),
        Frame::SimpleString(key),
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
