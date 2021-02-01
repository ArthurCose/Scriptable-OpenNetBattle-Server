mod net;
mod packets;
mod plugins;
mod threads;

use net::Server;

fn main() {
    let mut server = Server::new(8765);

    if let Err(err) = server.start() {
        panic!("{}", err);
    }
}
