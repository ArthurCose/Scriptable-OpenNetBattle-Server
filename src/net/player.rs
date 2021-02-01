use crate::packets::PacketShipper;

pub struct Player {
  pub socket_address: std::net::SocketAddr,
  pub packet_shipper: PacketShipper,
  pub id: String,
  pub area_id: String,
  pub avatar_id: u16,
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub ready: bool,
}
