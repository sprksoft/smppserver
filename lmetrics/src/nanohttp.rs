use std::{io::Read, net::TcpStream, time::Duration};

pub fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut full = Vec::with_capacity(100);
    loop {
        std::thread::sleep(Duration::from_millis(10));
        let mut buffer = [0u8; 100];
        let len = stream.read(&mut buffer)?;
        if len == 0 {
            break;
        }
        let buffer = &buffer[..len];
        full.extend_from_slice(buffer);
        if &buffer[len - 4..] == b"\r\n\r\n" {
            break;
        }
    }
    Ok(String::from_utf8(full).unwrap())
}

pub fn respond_404() -> &'static str {
    "HTTP/1.1 404 Not found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nnot found"
}

pub fn respond_200(data: String) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        data.as_bytes().len(),
        data
    )
}
