use itertools::Itertools;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:0")?;
    dbg!(&listener);

    let mut input = String::new();
    let mut stdout_buf = [0; 512];
    let mut stderr_buf = [0; 512];
    // accept connections and process them serially
    for mut stream in &listener.incoming().chunks(2) {
        let stream_write = stream.next().unwrap();
        let stream_read = stream.next().unwrap();
        let mut stream_write = stream_write?;
        let mut stream_read = stream_read?;
        stream_write.read_to_string(&mut input).unwrap();
        dbg!(&input);
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

        input.clear();
    }
    Ok(())
}
