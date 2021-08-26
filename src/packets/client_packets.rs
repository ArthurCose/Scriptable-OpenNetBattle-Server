// Increment VERSION_ITERATION src/packets/mod.rs if packets are added or modified

use super::bytes::*;
use super::management::{get_reliability, Reliability};
use super::PacketHeaders;
use crate::net::{BattleStats, Direction};

#[derive(Debug)]
pub enum ClientPacket {
  VersionRequest,
  Ack {
    reliability: Reliability,
    id: u64,
  },
  ServerMessage {
    data: Vec<u8>,
  },
  Heartbeat,
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
    identity: String,
    data: String,
  },
  Logout,
  RequestJoin,
  Ready {
    time: u64,
  },
  TransferredOut,
  Position {
    creation_time: u64,
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  },
  AvatarChange {
    name: String,
    element: String,
    max_health: u32,
  },
  Emote {
    emote_id: u8,
  },
  CustomWarp {
    tile_object_id: u32,
  },
  ObjectInteraction {
    tile_object_id: u32,
    button: u8,
  },
  ActorInteraction {
    actor_id: String,
    button: u8,
  },
  TileInteraction {
    x: f32,
    y: f32,
    z: f32,
    button: u8,
  },
  TextBoxResponse {
    response: u8,
  },
  PromptResponse {
    response: String,
  },
  BoardOpen,
  BoardClose,
  PostRequest,
  PostSelection {
    post_id: String,
  },
  ShopClose,
  ShopPurchase {
    item_name: String,
  },
  BattleResults {
    battle_stats: BattleStats,
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
    0 => Some(ClientPacket::VersionRequest),
    1 => Some(ClientPacket::Ack {
      reliability: get_reliability(read_byte(work_buf)?),
      id: read_u64(work_buf)?,
    }),
    // if this moves, check out message_server
    2 => Some(ClientPacket::ServerMessage {
      data: work_buf.to_vec(),
    }),
    3 => Some(ClientPacket::Heartbeat),
    4 => Some(ClientPacket::AssetFound {
      path: read_string_u16(work_buf)?,
      last_modified: read_u64(work_buf)?,
    }),
    5 => {
      let asset_type = read_byte(work_buf)?;
      let size = read_u16(work_buf)? as usize;
      let data = read_data(work_buf, size)?;

      Some(ClientPacket::AssetStream { asset_type, data })
    }
    6 => Some({
      let username = read_string_u8(work_buf)?;
      let identity_size = read_byte(work_buf)?;
      let identity_bytes = read_data(work_buf, identity_size as usize)?;
      let identity = base64::encode(&identity_bytes);
      let data = read_string_u16(work_buf)?;

      ClientPacket::Login {
        username,
        identity,
        data,
      }
    }),
    7 => Some(ClientPacket::Logout),
    8 => Some(ClientPacket::RequestJoin),
    9 => Some(ClientPacket::Ready {
      time: read_u64(work_buf)?,
    }),
    10 => Some(ClientPacket::TransferredOut),
    11 => Some(ClientPacket::Position {
      creation_time: read_u64(work_buf)?,
      x: read_f32(work_buf)?,
      y: read_f32(work_buf)?,
      z: read_f32(work_buf)?,
      direction: read_direction(read_byte(work_buf)?),
    }),
    12 => Some(ClientPacket::AvatarChange {
      name: read_string_u8(work_buf)?,
      element: read_string_u8(work_buf)?,
      max_health: read_u32(work_buf)?,
    }),
    13 => Some(ClientPacket::Emote {
      emote_id: read_byte(work_buf)?,
    }),
    14 => Some(ClientPacket::CustomWarp {
      tile_object_id: read_u32(work_buf)?,
    }),
    15 => Some(ClientPacket::ObjectInteraction {
      tile_object_id: read_u32(work_buf)?,
      button: read_byte(work_buf)?,
    }),
    16 => Some(ClientPacket::ActorInteraction {
      actor_id: read_string_u16(work_buf)?,
      button: read_byte(work_buf)?,
    }),
    17 => Some(ClientPacket::TileInteraction {
      x: read_f32(work_buf)?,
      y: read_f32(work_buf)?,
      z: read_f32(work_buf)?,
      button: read_byte(work_buf)?,
    }),
    18 => Some(ClientPacket::TextBoxResponse {
      response: read_byte(work_buf)?,
    }),
    19 => Some(ClientPacket::PromptResponse {
      response: read_string_u16(work_buf)?,
    }),
    20 => Some(ClientPacket::BoardOpen),
    21 => Some(ClientPacket::BoardClose),
    22 => Some(ClientPacket::PostRequest),
    23 => Some(ClientPacket::PostSelection {
      post_id: read_string_u16(work_buf)?,
    }),
    24 => Some(ClientPacket::ShopClose),
    25 => Some(ClientPacket::ShopPurchase {
      item_name: read_string_u8(work_buf)?,
    }),
    26 => Some(ClientPacket::BattleResults {
      battle_stats: BattleStats {
        health: read_u32(work_buf)?,
        score: read_u32(work_buf)?,
        time: read_f32(work_buf)?,
        ran: read_bool(work_buf)?,
        emotion: read_byte(work_buf)?,
      },
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
