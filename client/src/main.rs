use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::io::prelude::*;
use std::net::TcpStream;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let server = std::env::args()
        .nth(1)
        .ok_or("server address is required")?;
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut output = [0; 512];
    let mut data: Vec<u8> = vec![];
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
        rl.save_history("history.txt")?;

        stream_write.write_all(line.as_bytes())?;
        drop(stream_write);

        loop {
            let read_n = stream_read.read(&mut output)?;
            if output.starts_with(b"?vim") {
                data.clear();
                data.extend(&output[..read_n]);
                loop {
                    let read_n = stream_read.read(&mut output)?;
                    data.extend(&output[..read_n]);
                    if read_n == 0 {
                        break;
                    }
                }
                let orig_data = String::from_utf8_lossy(&data);
                let mut data = orig_data.splitn(2, "???");
                let file_name = data
                    .next()
                    .ok_or(format!("filename is missing, msg: {}", orig_data))?
                    .strip_prefix("?vim")
                    .ok_or(format!("?vim is missing, msg: {}", orig_data))?
                    .trim();
                let data = data
                    .next()
                    .ok_or(format!("data is missing, msg: {}", orig_data))?;

                let path = std::env::temp_dir().join(file_name);
                std::fs::write(&path, &data)?;

                std::process::Command::new("nvim")
                    .arg(&path)
                    .spawn()?
                    .wait()?;

                let mut stream_write = TcpStream::connect(&server)?;
                let _stream_read = TcpStream::connect(&server)?;

                let client_data = std::fs::read_to_string(&path)?;
                stream_write.write_all(client_data.as_bytes())?;

                break;
            }
            if read_n == 0 {
                break;
            }
            println!("{}", &String::from_utf8_lossy(&output[..read_n]));
        }
    }
}
