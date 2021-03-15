use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    let server = std::env::args().nth(1).unwrap();
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut output = [0; 512];
    loop {
        let mut stream_write = TcpStream::connect(&server)?;
        let mut stream_read = TcpStream::connect(&server)?;

        let line = loop {
            match rl.readline(">> ") {
                Ok(l) => break l,
                Err(ReadlineError::Interrupted) => {
                    continue;
                }
                _ => return Ok(()),
            }
        };
        rl.add_history_entry(line.as_str());
        rl.save_history("history.txt").unwrap();

        stream_write.write_all(line.as_bytes())?;
        drop(stream_write);
        loop {
            let read_n = stream_read.read(&mut output)?;
            if read_n == 0 {
                break;
            }
            println!("{}", &String::from_utf8(output[..read_n].to_vec()).unwrap());
        }
    }
}
