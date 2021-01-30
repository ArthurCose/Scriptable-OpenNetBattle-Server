use super::bytes::*;
use super::management::{get_reliability, Reliability};
use super::{PacketHeaders, TILE_HEIGHT, TILE_WIDTH};

#[derive(Debug)]
pub enum ClientPacket {
  Ping,
  Ack { reliability: Reliability, id: u64 },
  Login { username: String },
  Logout,
  LoadedMap { map_id: u64 },
  Position { x: f64, y: f64, z: f64 },
  AvatarChange { form_id: u16 },
  Emote { emote_id: u8 },
}

pub fn parse_client_packet(buf: &[u8]) -> Option<(PacketHeaders, ClientPacket)> {
  let mut work_buf = &buf[..];
  Some((parse_headers(&mut work_buf)?, parse_body(&mut work_buf)?))
}

fn parse_headers(work_buf: &mut &[u8]) -> Option<PacketHeaders> {
  let reliability_id = read_byte(work_buf)?;

  let id = if reliability_id > 0 {
    read_u64(work_buf)?
  } else {
    0
  };

  Some(PacketHeaders {
    reliability: get_reliability(reliability_id),
    id,
  })
}

fn parse_body(work_buf: &mut &[u8]) -> Option<ClientPacket> {
  match read_u16(work_buf)? {
    0 => Some(ClientPacket::Ping),
    1 => Some(ClientPacket::Ack {
      reliability: get_reliability(read_byte(work_buf)?),
      id: read_u64(work_buf)?,
    }),
    2 => Some(ClientPacket::Login {
      username: read_string(work_buf)?,
    }),
    3 => Some(ClientPacket::Logout),
    4 => Some(ClientPacket::LoadedMap {
      map_id: read_u64(work_buf)?,
    }),
    5 => Some(ClientPacket::Position {
      x: read_f64(work_buf)? / TILE_WIDTH * 2.0,
      y: read_f64(work_buf)? / TILE_HEIGHT,
      z: read_f64(work_buf)?,
    }),
    6 => Some(ClientPacket::AvatarChange {
      form_id: read_u16(work_buf)?,
    }),
    7 => Some(ClientPacket::Emote {
      emote_id: read_byte(work_buf)?,
    }),
    _ => None,
  }
}
