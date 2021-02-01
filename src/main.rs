mod net;
mod packets;
mod plugins;
mod threads;

use net::Server;
use plugins::LuaPluginInterface;

fn main() {
    let mut server = Server::new(8765);

    server.add_plugin_interface(Box::new(LuaPluginInterface::new()));

    if let Err(err) = server.start() {
        panic!("{}", err);
    }
}
