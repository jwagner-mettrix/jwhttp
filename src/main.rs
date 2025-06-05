use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::io::{Error};
use std::io::{BufReader, Write};
use std::time::Instant;

fn handle_client(mut stream: TcpStream) {
    let start = Instant::now();

    let mut reader = BufReader::new(&stream);

    let mut header = 0;
    let mut request_type = String::from("");
    let mut endpoint = String::from("");

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if header == 0 {
                    for (i, val) in line.split_whitespace().enumerate() {
                        match i {
                            0 => request_type = val.to_string(),
                            1 => endpoint = val.to_string(),
                            _ => ()
                        }
                    };
                }
                if line == "\r\n" { break }
            }
            Err(e) => panic!("IO error: {e}"),
        };
        header += 1;
    };

    let html = format!(
        "<html><head><title>jwhttp</title></head><body>jwhttp's {} response to {}</body></html>",
        request_type,
        endpoint
    );
    let html_bytes = html.len();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\n\r\n{}\r\n\r\n",
        html_bytes,
        html
    );
    
    match stream.write_fmt(format_args!("{response}")) {
        Ok(_) => (),
        Err(e) => panic!("encountered IO error: {e}"),
    }

    let duration = start.elapsed();

    println!("{request_type} {endpoint} - 200 {:?}", duration);
}

fn main() -> Result<(), Error> {
    let listener = TcpListener::bind("127.0.0.1:80")?;

    for stream in listener.incoming() {
        handle_client(stream?);
    }

    Ok(())
}
