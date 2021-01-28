mod net;
mod packets;
mod plugins;
mod server;
mod threads;
use server::Server;

fn main() {
    let mut server = Server::new();

    if let Err(err) = server.start() {
        panic!("{}", err);
    }
}
