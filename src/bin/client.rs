use std::{io::Read, net::TcpStream};

struct Client {
    connection: TcpStream,
}

impl Client {
    fn connect() -> Self {
        let connection = TcpStream::connect("localhost:6379").unwrap();
        Client { connection }
    }
}

fn main() {
    let mut client = Client::connect();

    let mut buffer = [0u8; 1024]; // Buffer to hold incoming data

    // Read data from the stream
    let bytes_read = client.connection.read(&mut buffer).unwrap();
    // Process the received data
    let received_data = String::from_utf8_lossy(&buffer[..bytes_read]);

    println!("Received: {:?}", received_data);
}
