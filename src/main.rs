use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::io::{Error};
use std::io::{BufReader, Write};
use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

struct Server {
    listener: TcpListener
}

impl Drop for Server {
    fn drop(&mut self) {
        println!("Server shutting down gracefully...");
    }
}

struct HttpRequest {
    request_type: String,
    endpoint: String
}

fn parse_request(mut reader: BufReader<&TcpStream>) -> HttpRequest {
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

    HttpRequest {
        request_type,
        endpoint
    }
}

fn handle_client(mut stream: TcpStream) {
    let start = Instant::now();

    let reader = BufReader::new(&stream);

    let HttpRequest {
        request_type,
        endpoint
    } = parse_request(reader);

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
    
    // Close the connection to prevent keep-alive hanging
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

fn main() -> Result<(), Error> {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn SetConsoleCtrlHandler(handler: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
        }
        
        unsafe extern "system" fn ctrl_handler(_: u32) -> i32 {
            SHUTDOWN.store(true, Ordering::Relaxed);
            1
        }
        
        unsafe {
            SetConsoleCtrlHandler(Some(ctrl_handler), 1);
        }
    }

    let server = Server {
        listener: TcpListener::bind("127.0.0.1:80")?
    };

    println!("Server started on http://127.0.0.1/");

    server.listener.set_nonblocking(true)?;
    
    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            println!("Received shutdown signal, stopping server...");
            break;
        }
        
        match server.listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                handle_client(stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
