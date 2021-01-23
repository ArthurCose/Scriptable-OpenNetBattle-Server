use crate::packets::ClientPacket;

pub enum ThreadMessage {
  Tick(Box<dyn FnOnce() -> () + Send>),
  ClientPacket(std::net::SocketAddr, ClientPacket),
}
