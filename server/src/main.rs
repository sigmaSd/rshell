use crossbeam_utils::thread;
use itertools::Itertools;
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:0")?;
    println!("server listener: {}", listener.local_addr()?);

    let sig_listener = TcpListener::bind("0.0.0.0:0")?;
    println!("signal listener: {}", sig_listener.local_addr()?);
    std::thread::spawn(move || -> std::io::Result<()> {
        for sig in sig_listener.incoming() {
            let mut sig = sig?;
            loop {
                let n = sig.read(&mut [0; 1])?;
                if n == 0 {
                    break;
                }
                Command::new("fish")
                    .arg("-c")
                    .arg("pkill -2  fish")
                    .spawn()?
                    .wait()?;
            }
        }
        Ok(())
    });

    let mut input = String::new();
    // accept connections and process them serially
    let listener = listener.incoming().chunks(2);
    let mut listener = listener.into_iter();
    loop {
        let mut stream = match listener.next() {
            Some(s) => s,
            None => break Ok(()),
        };
        let mut stream_write = stream.next().ok_or("client should send this stream")??;
        let mut stream_read = stream.next().ok_or("client should send this stream")??;
        stream_write.read_to_string(&mut input)?;

        dbg!(&input);

        if input.starts_with("cd") {
            let dir = input.split_whitespace().nth(1);
            if let Some(dir) = dir {
                std::env::set_current_dir(dir)?;
            }
        } else if input.starts_with("vim") {
            let file = Path::new(input.strip_prefix("vim").expect("already checked").trim());
            let file_name = file
                .file_name()
                .ok_or("Could not read filename")?
                .to_str()
                .ok_or("Could not read filename")?;

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
            std::fs::write(&file, client_data)?;
        } else {
            let mut process = Command::new("fish")
                .arg("-c")
                .arg(&input)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut stdout = process.stdout.take().expect("stdout is piped");
            let mut stderr = process.stderr.take().expect("stderr is piped");
            let (tx, rx) = mpsc::channel();

            let read_process_and_write_to_stream =
                |out: &mut dyn Read, tx: mpsc::Sender<Vec<u8>>| -> std::io::Result<bool> {
                    let mut out_buf = [0; 512];
                    let out_n = out.read(&mut out_buf)?;
                    if out_n != 0 {
                        tx.send(out_buf[..out_n].to_vec()).map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "failed to send data to write thread",
                            )
                        })?;
                    }
                    Ok(out_n == 0)
                };

            thread::scope(move |s| {
                let tx_c = tx.clone();
                s.spawn(move |_| {
                    while let Ok(false) =
                        read_process_and_write_to_stream(&mut stdout, tx_c.clone())
                    {
                    }
                });
                s.spawn(move |_| {
                    while let Ok(false) = read_process_and_write_to_stream(&mut stderr, tx.clone())
                    {
                    }
                });
                s.spawn(move |_| -> std::io::Result<()> {
                    while let Ok(data) = rx.recv() {
                        stream_read.write_all(&data)?;
                    }
                    Ok(())
                });
            })
            .map_err(|e| format!("read/write threads failed {:?}", e))?;
        }

        input.clear();
    }
}
