// Uncomment this block to pass the first stage
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Err(e) => {
                eprintln!("failed to read stream: {}", e);
                break;
            }

            Ok(mut stream) => {
                handle_connection(&mut stream);
            }
        }
    }
}

fn handle_connection(stream: &mut TcpStream) {
    let mut buffer = String::new();

    match stream.read_to_string(&mut buffer) {
        Ok(_) => handle_command(&buffer, stream),
        Err(e) => {
            eprintln!("failed to read stream: {}", e);
        }
    }
}

fn handle_command(_buffer: &str, stream: &mut TcpStream) {
    let pong = "+PONG\r\n".as_bytes();

    stream.write_all(pong).unwrap();

    stream.flush().unwrap();
}
