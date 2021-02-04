use super::Navi;
use crate::packets::PacketShipper;
use std::collections::HashSet;

pub struct Player {
  pub socket_address: std::net::SocketAddr,
  pub packet_shipper: PacketShipper,
  pub navi: Navi,
  pub ready: bool,
  pub cached_assets: HashSet<String>,
}
