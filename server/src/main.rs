use itertools::Itertools;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:0")?;
    dbg!(&listener);

    let mut input = String::new();
    let mut stdout_buf = [0; 512];
    let mut stderr_buf = [0; 512];
    // accept connections and process them serially
    let listener = listener.incoming().chunks(2);
    let mut listener = listener.into_iter();
    loop {
        let mut stream = match listener.next() {
            Some(s) => s,
            None => break Ok(()),
        };
        let stream_write = stream.next().ok_or("client should send this stream")?;
        let stream_read = stream.next().ok_or("client should send this stream")?;
        let mut stream_write = stream_write?;
        let mut stream_read = stream_read?;
        stream_write.read_to_string(&mut input)?;

        dbg!(&input);

        if input.starts_with("cd") {
            let dir = input.split_whitespace().nth(1);
            if let Some(dir) = dir {
                std::env::set_current_dir(dir).unwrap();
            }
        } else if input.starts_with("vim") {
            let file = input.strip_prefix("vim").unwrap();
            let file_name = std::path::Path::new(file)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let data = std::fs::read_to_string(&file).unwrap_or_default();
            stream_read.write_all(b"?vim")?;
            stream_read.flush()?;
            stream_read.write_all(file_name.as_bytes())?;
            stream_read.write_all(b"???")?;
            stream_read.write_all(data.as_bytes())?;
            drop(stream_read);
            // get result
            let mut stream = listener.next().ok_or("client should send this stream")?;
            let mut stream_write = stream.next().ok_or("client should send this stream")??;
            let _stream_read = stream.next().ok_or("client should send this stream")??;
            let mut client_data = String::new();
            stream_write.read_to_string(&mut client_data)?;
            std::fs::write(&file.trim(), client_data)?;
        } else {
            let mut process = Command::new("fish")
                .arg("-c")
                .arg(&input)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut stdout = process.stdout.take().unwrap();
            let mut stderr = process.stderr.take().unwrap();

            let mut read_process_and_write_to_stream = || -> std::io::Result<bool> {
                let stdout_n = stdout.read(&mut stdout_buf)?;
                let stderr_n = stderr.read(&mut stderr_buf)?;
                if stdout_n != 0 {
                    stream_read.write_all(&stdout_buf[..stdout_n])?;
                }
                if stderr_n != 0 {
                    stream_read.write_all(&stderr_buf[..stderr_n])?;
                }
                Ok(stdout_n == 0 && stderr_n == 0)
            };

            while let Ok(false) = read_process_and_write_to_stream() {}
        }

        input.clear();
    }
}
