use crate::packets::{ClientPacket, PacketHeaders};

pub enum ThreadMessage {
  Tick(Box<dyn FnOnce() + Send>),
  ClientPacket {
    socket_address: std::net::SocketAddr,
    headers: PacketHeaders,
    packet: ClientPacket,
  },
}
