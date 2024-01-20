use mini_redis::{connection::Connection, frame::Frame};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();

        let mut connection = Connection::new(stream);
        match connection.read_frame().await {
            Ok(frame) => {
                println!("{frame:?}");
                match frame {
                    Some(_frame) => connection
                        .write(Frame::String("output".to_string()))
                        .await
                        .unwrap(),
                    None => connection
                        .write(Frame::String("none".to_string()))
                        .await
                        .unwrap(),
                }
            }
            Err(_err) => connection
                .write(Frame::String("err".to_string()))
                .await
                .unwrap(),
        }
    }
}
