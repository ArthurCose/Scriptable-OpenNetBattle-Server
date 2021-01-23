use crate::packets::ClientPacket;

pub enum ThreadMessage {
  Tick,
  ClientPacket(std::net::SocketAddr, ClientPacket),
}
