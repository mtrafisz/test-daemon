use std::io::prelude::*;
use std::net::TcpStream;

mod shared;
use shared::PORT;

fn main() {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", PORT)).expect("Could not connect to server");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("Could not read from server");
    println!("{}", response);
}
