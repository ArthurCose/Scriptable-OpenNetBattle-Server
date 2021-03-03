// Increment VERSION_ITERATION src/packets/mod.rs if packets are added or modified

use super::bytes::*;
use super::{VERSION_ID, VERSION_ITERATION};
use crate::net::{Asset, AssetData};

#[derive(Debug)]
pub enum ServerPacket<'a> {
  Pong {
    max_payload_size: usize,
  },
  Ack {
    reliability: u8,
    id: u64,
  },
  Login {
    ticket: String,
  },
  AssetStream {
    data: &'a [u8],
  },
  AssetStreamComplete {
    name: String,
    asset: &'a Asset,
  },
  MapUpdate {
    map_path: String,
  },
  TransferStart,
  TransferComplete,
  NaviConnected {
    ticket: String,
    name: String,
    texture_path: String,
    animation_path: String,
    x: f32,
    y: f32,
    z: f32,
    warp_in: bool,
  },
  NaviDisconnected {
    ticket: String,
    warp_out: bool,
  },
  NaviSetName {
    ticket: String,
    name: String,
  },
  NaviMove {
    ticket: String,
    x: f32,
    y: f32,
    z: f32,
  },
  NaviSetAvatar {
    ticket: String,
    texture_path: String,
    animation_path: String,
  },
  NaviEmote {
    ticket: String,
    emote_id: u8,
  },
}

pub fn build_unreliable_packet(packet: &ServerPacket) -> Vec<u8> {
  let mut buf = build_packet(packet);
  buf.insert(0, 0);
  buf
}

pub(super) fn build_packet(packet: &ServerPacket) -> Vec<u8> {
  let mut buf = Vec::new();

  match packet {
    ServerPacket::Pong { max_payload_size } => {
      write_u16(&mut buf, 0);
      write_str(&mut buf, VERSION_ID);
      write_u64(&mut buf, VERSION_ITERATION);
      write_u16(&mut buf, *max_payload_size as u16);
    }
    ServerPacket::Ack { reliability, id } => {
      write_u16(&mut buf, 1);
      buf.push(*reliability);
      write_u64(&mut buf, *id);
    }
    ServerPacket::Login { ticket } => {
      write_u16(&mut buf, 2);
      write_string(&mut buf, ticket);
    }
    ServerPacket::AssetStream { data } => {
      write_u16(&mut buf, 3);
      write_u16(&mut buf, data.len() as u16);
      write_data(&mut buf, data);
    }
    ServerPacket::AssetStreamComplete { name, asset } => {
      write_u16(&mut buf, 4);
      write_string(&mut buf, name);

      match asset.data {
        AssetData::Text(_) => {
          buf.push(0);
        }
        AssetData::Texture(_) => {
          buf.push(1);
        }
        AssetData::Audio(_) => {
          buf.push(2);
        }
        AssetData::SFMLImage(_) => {
          buf.push(3);
        }
      }
    }
    ServerPacket::MapUpdate { map_path } => {
      write_u16(&mut buf, 5);
      write_string(&mut buf, map_path);
    }
    ServerPacket::TransferStart => {
      write_u16(&mut buf, 6);
    }
    ServerPacket::TransferComplete => {
      write_u16(&mut buf, 7);
    }
    ServerPacket::NaviConnected {
      ticket,
      name,
      texture_path,
      animation_path,
      x,
      y,
      z,
      warp_in,
    } => {
      write_u16(&mut buf, 8);
      write_string(&mut buf, ticket);
      write_string(&mut buf, name);
      write_string(&mut buf, texture_path);
      write_string(&mut buf, animation_path);
      write_f32(&mut buf, *x);
      write_f32(&mut buf, *y);
      write_f32(&mut buf, *z);
      write_bool(&mut buf, *warp_in);
    }
    ServerPacket::NaviDisconnected { ticket, warp_out } => {
      write_u16(&mut buf, 9);
      write_string(&mut buf, ticket);
      write_bool(&mut buf, *warp_out);
    }
    ServerPacket::NaviSetName { ticket, name } => {
      write_u16(&mut buf, 10);
      write_string(&mut buf, ticket);
      write_string(&mut buf, name);
    }
    ServerPacket::NaviMove { ticket, x, y, z } => {
      write_u16(&mut buf, 11);
      write_string(&mut buf, ticket);
      write_f32(&mut buf, *x);
      write_f32(&mut buf, *y);
      write_f32(&mut buf, *z);
    }
    ServerPacket::NaviSetAvatar {
      ticket,
      texture_path,
      animation_path,
    } => {
      write_u16(&mut buf, 12);
      write_string(&mut buf, ticket);
      write_string(&mut buf, texture_path);
      write_string(&mut buf, animation_path);
    }
    ServerPacket::NaviEmote { ticket, emote_id } => {
      write_u16(&mut buf, 13);
      buf.push(*emote_id);
      write_string(&mut buf, ticket);
    }
  }

  buf
}

pub fn create_asset_stream<'a>(
  max_payload_size: usize,
  name: &String,
  asset: &'a Asset,
) -> Vec<ServerPacket<'a>> {
  // reliability type + id + packet type + data size
  const HEADER_SIZE: usize = 1 + 8 + 2 + 2 + 16;

  let mut packets = Vec::new();

  let mut bytes = match &asset.data {
    AssetData::Text(data) => data.as_bytes(),
    AssetData::Texture(data) => &data,
    AssetData::Audio(data) => &data,
    AssetData::SFMLImage(data) => &data,
  };

  let mut remaining_bytes = bytes.len();

  while remaining_bytes > 0 {
    let available_room = max_payload_size - HEADER_SIZE;
    let size = if remaining_bytes < available_room {
      remaining_bytes
    } else {
      available_room
    };

    packets.push(ServerPacket::AssetStream {
      data: &bytes[..size],
    });

    bytes = &bytes[size..];

    remaining_bytes -= size;
  }

  packets.push(ServerPacket::AssetStreamComplete {
    name: name.clone(),
    asset,
  });

  packets
}
