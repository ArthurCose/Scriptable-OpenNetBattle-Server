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
  ServerMessage {
    data: Vec<u8>,
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
    data: String,
  },
  Logout,
  RequestJoin,
  Ready,
  TransferredOut,
  Position {
    creation_time: u64,
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  },
  AvatarChange,
  Emote {
    emote_id: u8,
  },
  CustomWarp {
    tile_object_id: u32,
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
  TextBoxResponse {
    response: u8,
  },
  BoardOpen,
  BoardClose,
  PostRequest,
  PostSelection {
    post_id: String,
  },
}

pub fn parse_client_packet(buf: &[u8]) -> Option<(PacketHeaders, ClientPacket)> {
  let mut work_buf = buf;
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
    // if this moves, check out poll_server
    0 => Some(ClientPacket::Ping),
    1 => Some(ClientPacket::Ack {
      reliability: get_reliability(read_byte(work_buf)?),
      id: read_u64(work_buf)?,
    }),
    // if this moves, check out message_server
    2 => Some(ClientPacket::ServerMessage {
      data: work_buf.to_vec(),
    }),
    3 => Some(ClientPacket::AssetFound {
      path: read_string(work_buf)?,
      last_modified: read_u64(work_buf)?,
    }),
    4 => {
      let asset_type = read_byte(work_buf)?;
      let size = read_u16(work_buf)? as usize;
      let data = read_data(work_buf, size)?;

      Some(ClientPacket::AssetStream { asset_type, data })
    }
    5 => Some(ClientPacket::Login {
      username: read_string(work_buf)?,
      data: read_string(work_buf)?,
    }),
    6 => Some(ClientPacket::Logout),
    7 => Some(ClientPacket::RequestJoin),
    8 => Some(ClientPacket::Ready),
    9 => Some(ClientPacket::TransferredOut),
    10 => Some(ClientPacket::Position {
      creation_time: read_u64(work_buf)?,
      x: read_f32(work_buf)?,
      y: read_f32(work_buf)?,
      z: read_f32(work_buf)?,
      direction: read_direction(read_byte(work_buf)?),
    }),
    11 => Some(ClientPacket::AvatarChange),
    12 => Some(ClientPacket::Emote {
      emote_id: read_byte(work_buf)?,
    }),
    13 => Some(ClientPacket::CustomWarp {
      tile_object_id: read_u32(work_buf)?,
    }),
    14 => Some(ClientPacket::ObjectInteraction {
      tile_object_id: read_u32(work_buf)?,
    }),
    15 => Some(ClientPacket::ActorInteraction {
      actor_id: read_string(work_buf)?,
    }),
    16 => Some(ClientPacket::TileInteraction {
      x: read_f32(work_buf)?,
      y: read_f32(work_buf)?,
      z: read_f32(work_buf)?,
    }),
    17 => Some(ClientPacket::TextBoxResponse {
      response: read_byte(work_buf)?,
    }),
    18 => Some(ClientPacket::BoardOpen),
    19 => Some(ClientPacket::BoardClose),
    20 => Some(ClientPacket::PostRequest),
    21 => Some(ClientPacket::PostSelection {
      post_id: read_string(work_buf)?,
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
