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
    let mut data: Vec<u8> = vec![];
    let mut vim_flag = false;
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
            if output.starts_with(b"?vim") {
                data.clear();
                data.extend(&output[..read_n]);
                vim_flag = true;
                loop {
                    let read_n = stream_read.read(&mut output)?;
                    data.extend(&output[..read_n]);
                    if read_n == 0 {
                        break;
                    }
                }
            }
            if read_n == 0 {
                break;
            }
            if vim_flag {
                let data = String::from_utf8(data.clone()).unwrap();
                let mut data = data.splitn(2, "???");
                let file_name = data.next().unwrap().strip_prefix("?vim").unwrap().trim();
                let data = data.next().unwrap();

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

                vim_flag = false;
            } else {
                println!("{}", &String::from_utf8_lossy(&output[..read_n]));
            }
        }
    }
}
