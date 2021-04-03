pub mod bytes;

mod client_packets;
pub use client_packets::*;

mod server_packets;
pub use server_packets::*;

mod management;
pub use management::*;

pub struct PacketHeaders {
  pub reliability: Reliability,
  pub id: u64,
}

const VERSION_ID: &str = "https://github.com/ArthurCose/Scriptable-OpenNetBattle-Server";
const VERSION_ITERATION: u64 = 8;
