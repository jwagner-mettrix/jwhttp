use std::{
    io::Error, net::{TcpListener, TcpStream}
};
use crate::threads::ThreadPool;

pub struct Server {
    pub listener: TcpListener,
    pub pool: ThreadPool,
}

impl Server {
    pub fn new(ip_port: &str) -> Result<Server, Error> {
        let listener = TcpListener::bind(ip_port)?;
        let pool = ThreadPool::new(4);
        println!("Server started on http://{ip_port}/");
        Ok(Server {
            listener,
            pool
        })
    }

    pub fn listen(&self, handle_client: fn(TcpStream)) -> Result<(), Error> {
        // set listener to non-blocking so we can check shutdown flag
        self.listener.set_nonblocking(true)?;
        
        loop {
            // if SHUTDOWN.load(Ordering::Relaxed) {
            //     println!("Received shutdown signal, stopping server...");
            //     break;
            // }
            
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

        // Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        println!("Server shutting down gracefully...");
    }
}
