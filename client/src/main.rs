use std::io;
use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    let server = std::env::args().nth(1).unwrap();

    let mut input = String::new();
    let mut output = [0; 512];
    loop {
        let mut stream_write = TcpStream::connect(&server)?;
        let mut stream_read = TcpStream::connect(&server)?;
        io::stdin().read_line(&mut input)?;
        stream_write.write_all(input.as_bytes())?;
        drop(stream_write);
        loop {
            let read_n = stream_read.read(&mut output)?;
            if read_n == 0 {
                break;
            }
            println!("{}", &String::from_utf8(output[..read_n].to_vec()).unwrap());
        }
        input.clear();
    }
}
