use std::{io::Write, net::TcpListener};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for (index, stream) in listener.incoming().enumerate() {
        let mut stream = stream.unwrap();

        stream
            .write_all(format!("some value, {}", index).as_bytes())
            .unwrap();
        stream.flush().unwrap();
    }
}
