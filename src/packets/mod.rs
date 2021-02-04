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

pub const MAX_BUFFER_LEN: usize = 10240;

pub(super) const VERSION_ID: &str =
  "https://github.com/ArthurCose/Scriptable-OpenNetBattle-Server/tree/proposed-packets";
pub(super) const VERSION_ITERATION: u64 = 2;

pub(super) const TILE_WIDTH: f32 = 62.0 + 2.5;
pub(super) const TILE_HEIGHT: f32 = 32.0 + 0.5;
