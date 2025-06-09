use std::{
    io::{Error, BufReader, BufRead, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicBool, Ordering}
};
use crate::threads::ThreadPool;
use crate::parser::HttpRequest;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);
const NOT_FOUND_RESPONSE: &str = "HTTP/1.1 404 NOT FOUND\r\nConnection: close\r\n\r\n";
const BAD_REQUEST_RESPONSE: &str = "HTTP/1.1 400 BAD REQUEST\r\nConnection: close\r\n\r\n";

pub struct Server {
    pub listener: TcpListener,
    pub pool: ThreadPool,
}

impl Server {
    pub fn new(ip_port: &str) -> Result<Server, Error> {

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

        let listener = TcpListener::bind(ip_port)?;
        let pool = ThreadPool::new(4);
        println!("Server started on http://{ip_port}/");
        Ok(Server {
            listener,
            pool
        })
    }

    fn handle_client(mut stream: TcpStream) {
        // Set a short read timeout so we don't block indefinitely
        stream.set_read_timeout(Some(std::time::Duration::from_millis(500))).ok();
        
        // Keep-alive loop - handle multiple requests on the same connection
        loop {
            // Check shutdown flag at the start of each request
            if SHUTDOWN.load(Ordering::Relaxed) {
                return;
            }
            
            let reader = BufReader::new(&stream);
            let mut http_request = Vec::new();
            let mut bad_request = false;
            
            // Read lines and handle errors
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        if line.is_empty() {
                            break; // End of HTTP headers
                        }
                        http_request.push(line);
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                        bad_request = true;
                        break;
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                        bad_request = true;
                        break;
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        // Timeout is expected in keep-alive scenarios - just exit this connection
                        break;
                    },
                    Err(e) if e.raw_os_error() == Some(10060) => {
                        // Windows timeout error - same as TimedOut  
                        break;
                    },
                    Err(e) => {
                        eprintln!("Warning: Error reading request: {}", e);
                        bad_request = true;
                        break;
                    }
                }
            }
            
            let (HttpRequest {
                method,
                path,
                host,
                version,
                connection,
                accept,
                params,
                headers,
                bad_request: parse_bad_request,
            }, request_start) = HttpRequest::new(http_request);
    
            // Check shutdown flag again before generating response
            if SHUTDOWN.load(Ordering::Relaxed) {
                return;
            }
    
            // If there was an error reading the request, close the connection
            if bad_request || parse_bad_request {
                let _ = stream.write_fmt(format_args!("{}", BAD_REQUEST_RESPONSE));
                return;
            }
            
            // If method is empty, it likely means we hit a timeout or connection closed
            if method.is_empty() {
                return;
            }
    
            let html = format!(
                "<html><head><title>jwhttp</title></head><body>jwhttp's {} response to {}</body></html>",
                method,
                path
            );
            let html_bytes = html.len();
            
            let should_keep_alive = connection.to_lowercase().contains("keep-alive") && !SHUTDOWN.load(Ordering::Relaxed);
            let connection_header = if should_keep_alive { "keep-alive" } else { "close" };
            
            let response = if path == "/favicon.ico" {
                format!("{NOT_FOUND_RESPONSE}\r\nConnection: {}\r\n\r\n", connection_header)
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nConnection: {}\r\nContent-Length: {}\r\n\r\n{}\r\n\r\n",
                    connection_header,
                    html_bytes,
                    html
                )
            };
            
            match stream.write_fmt(format_args!("{response}")) {
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                    return;
                },
                Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                    return;
                },
                Err(e) if e.raw_os_error() == Some(10093) => {
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
            
            // Exit the loop if we shouldn't keep the connection alive
            if !should_keep_alive {
                return;
            }
            
            // Continue to next request on this connection
        }
    }

    pub fn listen(&self) -> Result<(), Error> {
        // set listener to non-blocking so we can check shutdown flag
        self.listener.set_nonblocking(true)?;
        
        loop {
            if SHUTDOWN.load(Ordering::Relaxed) {
                println!("Received shutdown signal, stopping server...");
                break;
            }
            
            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    self.pool.execute(move || {
                        match stream.set_nonblocking(false) {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("Failed to set stream blocking: {}", e);
                                panic!("Quitting due to skill issues surrounding the handling of nonblocking threads.");
                            }
                        }
                        Server::handle_client(stream);
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
}

impl Drop for Server {
    fn drop(&mut self) {
        println!("Server shutting down gracefully...");
    }
}
