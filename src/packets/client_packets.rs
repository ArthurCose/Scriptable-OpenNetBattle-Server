use super::bytes::*;
use super::{TILE_HEIGHT, TILE_WIDTH};

#[derive(Debug)]
pub enum ClientPacket {
  Ping,
  Login { username: String },
  Position { x: f64, y: f64, z: f64 },
  Logout,
  LoadedMap { map_id: u64 },
  AvatarChange { form_id: u16 },
  Emote { emote_id: u8 },
}

pub fn parse_client_packet(buf: &[u8]) -> Option<ClientPacket> {
  let mut work_buf = &buf[..];

  match read_u16(&mut work_buf)? {
    0 => Some(ClientPacket::Login {
      username: read_string(&mut work_buf)?,
    }),
    1 => Some(ClientPacket::Position {
      x: read_f64(&mut work_buf)? / TILE_WIDTH * 2.0,
      y: read_f64(&mut work_buf)? / TILE_HEIGHT,
      z: read_f64(&mut work_buf)?,
    }),
    2 => Some(ClientPacket::Logout),
    3 => Some(ClientPacket::LoadedMap {
      map_id: read_u64(&mut work_buf)?,
    }),
    4 => Some(ClientPacket::AvatarChange {
      form_id: read_u16(&mut work_buf)?,
    }),
    5 => Some(ClientPacket::Emote {
      emote_id: read_byte(&mut work_buf)?,
    }),
    6 => Some(ClientPacket::Ping),
    _ => None,
  }
}
