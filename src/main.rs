use jwhttp::Server;
fn main() {

    let server = match Server::new("127.0.0.1:80") {
        Ok(server) => server,
        Err(e) => panic!("Failed to start server: {e}")
    };

    server.listen().expect("Failed to listen on server");

}
