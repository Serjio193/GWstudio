use std::io::{BufReader, Read};
use std::sync::mpsc;
use std::thread;

pub(crate) fn forward_stream_updates<R: Read + Send + 'static>(
    reader: R,
    sender: mpsc::Sender<(bool, String)>,
    is_stdout: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = [0_u8; 1024];
        let mut pending = Vec::<u8>::new();

        loop {
            let bytes_read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => size,
                Err(_) => break,
            };

            for byte in &buffer[..bytes_read] {
                if *byte == b'\r' || *byte == b'\n' {
                    if !pending.is_empty() {
                        let line = String::from_utf8_lossy(&pending).trim().to_string();
                        if !line.is_empty() {
                            let _ = sender.send((is_stdout, line));
                        }
                        pending.clear();
                    }
                } else {
                    pending.push(*byte);
                }
            }
        }

        if !pending.is_empty() {
            let line = String::from_utf8_lossy(&pending).trim().to_string();
            if !line.is_empty() {
                let _ = sender.send((is_stdout, line));
            }
        }
    })
}
