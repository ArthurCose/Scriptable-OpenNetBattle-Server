// Increment VERSION_ITERATION src/packets/mod.rs if packets are added or modified

use super::bytes::*;
use super::management::{get_reliability, Reliability};
use super::PacketHeaders;
use crate::net::Direction;

#[derive(Debug)]
pub enum ClientPacket {
  Ping,
  Ack {
    reliability: Reliability,
    id: u64,
  },
  AssetFound {
    path: String,
    last_modified: u64,
  },
  AssetStream {
    asset_type: u8,
    data: Vec<u8>,
  },
  Login {
    username: String,
    password: String,
  },
  Logout,
  RequestJoin,
  Ready,
  Position {
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  },
  AvatarChange,
  Emote {
    emote_id: u8,
  },
  ObjectInteraction {
    tile_object_id: u32,
  },
  ActorInteraction {
    actor_id: String,
  },
  TileInteraction {
    x: f32,
    y: f32,
    z: f32,
  },
  DialogResponse {
    response: u8,
  },
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
    2 => Some(ClientPacket::AssetFound {
      path: read_string(work_buf)?,
      last_modified: read_u64(work_buf)?,
    }),
    3 => {
      let asset_type = read_byte(work_buf)?;
      let size = read_u16(work_buf)? as usize;
      let data = read_data(work_buf, size)?;

      Some(ClientPacket::AssetStream { asset_type, data })
    }
    4 => Some(ClientPacket::Login {
      username: read_string(work_buf)?,
      password: read_string(work_buf)?,
    }),
    5 => Some(ClientPacket::Logout),
    6 => Some(ClientPacket::RequestJoin),
    7 => Some(ClientPacket::Ready),
    8 => Some(ClientPacket::Position {
      x: read_f32(work_buf)?,
      y: read_f32(work_buf)?,
      z: read_f32(work_buf)?,
      direction: read_direction(read_byte(work_buf)?),
    }),
    9 => Some(ClientPacket::AvatarChange),
    10 => Some(ClientPacket::Emote {
      emote_id: read_byte(work_buf)?,
    }),
    11 => Some(ClientPacket::ObjectInteraction {
      tile_object_id: read_u32(work_buf)?,
    }),
    12 => Some(ClientPacket::ActorInteraction {
      actor_id: read_string(work_buf)?,
    }),
    13 => Some(ClientPacket::TileInteraction {
      x: read_f32(work_buf)?,
      y: read_f32(work_buf)?,
      z: read_f32(work_buf)?,
    }),
    14 => Some(ClientPacket::DialogResponse {
      response: read_byte(work_buf)?,
    }),
    _ => None,
  }
}

fn read_direction(byte: u8) -> Direction {
  match byte {
    0x01 => Direction::Up,
    0x02 => Direction::Left,
    0x04 => Direction::Down,
    0x08 => Direction::Right,
    0x10 => Direction::UpLeft,
    0x20 => Direction::UpRight,
    0x40 => Direction::DownLeft,
    0x80 => Direction::DownRight,
    _ => Direction::None,
  }
}
