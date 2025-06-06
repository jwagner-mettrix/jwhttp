use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::io::{Error};
use std::io::{BufReader, Write};
use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::thread;

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
    method: String,
    path: String,
    host: String,
    version: String,
    connection: String,
    accept: Vec<String>,
    params: HashMap<String, String>,
    headers: HashMap<String, String>,
    bad_request: bool
}

const NOT_FOUND_RESPONSE: &str = "HTTP/1.1 404\r\nConnection: close\r\n\r\n";
const BAD_REQUEST_RESPONSE: &str = "HTTP/1.1 404\r\nConnection: close\r\n\r\n";

fn parse_params(params_string: &str) -> HashMap<String, String> {
    let mut params: HashMap<String, String> = HashMap::new();

    let params_iter = params_string.split("&");

    for param in params_iter {
        let param_option = param.split_once("=");
        let (key, pair) = match param_option {
            Some((key, pair)) => (key, pair),
            _ => ("", "")
        };

        let key_clean = key.trim().to_string();
        let pair_clean = pair.trim().to_string();

        if !key_clean.is_empty() && !pair_clean.is_empty() {
            params.insert(key_clean.clone(), pair_clean.clone());
        }
    }

    params
}

fn parse_accept(accept_string: &str) -> Vec<String> {
    let mut accept = vec![];
    let accept_iter = accept_string.split(",");

    for accept_type in accept_iter {
        accept.push(accept_type.to_string());
    }

    accept
} 

fn parse_request(mut reader: BufReader<&TcpStream>) -> (HttpRequest, Instant) {
    let mut method = String::from("");
    let mut path = String::from("");
    let mut host: String = String::from("");
    let mut version: String = String::from("");
    let mut connection: String = String::from("");
    let mut accept: Vec<String> = vec![];
    let mut params: HashMap<String, String> = HashMap::new();
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut bad_request: bool = false;
    
    let mut count = 0;
    let mut request_start: Option<Instant> = None;
    
    loop {
        let mut line = String::new();
        
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if request_start.is_none() {
                    request_start = Some(Instant::now());
                }
                
                if line == "\r\n" { break }

                if count == 0 {
                    for (i, val) in line.split_whitespace().enumerate() {
                        match i {
                            0 => method = val.to_string(),
                            1 => {
                                let path_option = val.split_once("?");
                                let (_, params_string) = match path_option {
                                    Some((path_string, params_string)) => {
                                        path = path_string.to_string();
                                        (path_string, params_string)
                                    },
                                    _ => {
                                        path = val.to_string();
                                        ("", "")
                                    }
                                };
                                if !params_string.is_empty() {
                                    params = parse_params(&params_string);
                                }
                            },
                            2 => version = val.to_string(),
                            _ => ()
                        }
                    };
                } else {
                    let key_pair = line.split_once(":");
                
                    let (key, pair) = match key_pair {
                        Some((key,pair)) => (key, pair),
                        _ => {
                            bad_request = true;
                            ("", "")
                        }
                    };

                    let key_clean = key.trim().to_string();
                    let pair_clean = pair.trim().to_string();

                    headers.insert(key_clean.clone(), pair_clean.clone());

                    match key_clean.as_str() {
                        "Host" => host = pair_clean,
                        "Connection" => connection = pair_clean,
                        "Accept" => accept = parse_accept(&pair_clean),
                        _ => ()
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                bad_request = true;
                break;
            },
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                bad_request = true;
                break;
            },
            Err(e) => {
                eprintln!("Warning: Error reading request: {}", e);
                bad_request = true;
                break;
            }
        };
        count += 1;
    };

    (HttpRequest {
        method,
        path,
        host,
        version,
        connection,
        accept,
        params,
        headers,
        bad_request,
    }, request_start.unwrap_or(Instant::now()))
}

fn handle_client(mut stream: TcpStream) {
    let reader = BufReader::new(&stream);
    
    let (HttpRequest {
        method,
        path,
        host,
        version,
        connection,
        accept,
        params,
        headers,
        bad_request,
    }, request_start) = parse_request(reader);

    let html = format!(
        "<html><head><title>jwhttp</title></head><body>jwhttp's {} response to {}</body></html>",
        method,
        path
    );
    let html_bytes = html.len();
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}\r\n\r\n",
        html_bytes,
        html
    );

    if path == "/favicon.ico" { response = NOT_FOUND_RESPONSE.to_string() }
    if bad_request { response = BAD_REQUEST_RESPONSE.to_string() }
    
    match stream.write_fmt(format_args!("{response}")) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
            // connection was closed (likely during server shutdown)
            return;
        },
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
            // client disconnected
            return;
        },
        Err(e) if e.raw_os_error() == Some(10093) => {
            // windows networking shutdown during process exit
            return;
        },
        Err(e) => {
            eprintln!("Warning: Failed to send response: {}", e);
            return;
        }
    }

    let total_duration = request_start.elapsed();

    println!("----------------------------------------------");
    println!("{host} {method} {path} - 200 {:?}", total_duration);
    println!("{version} {connection}");
    println!("headers: {:?}", headers.keys());
    println!("params: {:?}", params);
    println!("accepts: {:?}", accept);
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

    println!("Server started on http://127.0.0.1:80/");

    // Set listener to non-blocking so we can check shutdown flag
    server.listener.set_nonblocking(true)?;
    
    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            println!("Received shutdown signal, stopping server...");
            break;
        }
        
        match server.listener.accept() {
            Ok((stream, _addr)) => {
                thread::spawn(move || {
                    match stream.set_nonblocking(false) {
                        Ok(_) => (),
                        _ => panic!("error")
                    }
                    handle_client(stream);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
