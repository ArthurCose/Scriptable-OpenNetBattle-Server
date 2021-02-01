// Increment VERSION_ITERATION src/packets/mod.rs if packets are added or modified

use super::bytes::*;
use super::{TILE_HEIGHT, TILE_WIDTH, VERSION_ID, VERSION_ITERATION};

pub enum ServerPacket {
  Pong,
  Ack {
    reliability: u8,
    id: u64,
  },
  Login {
    ticket: String,
  },
  MapData {
    map_data: String,
  },
  NaviConnected {
    ticket: String,
    name: String,
    x: f32,
    y: f32,
    z: f32,
    warp_in: bool,
  },
  NaviDisconnected {
    ticket: String,
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
    avatar_id: u16,
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
    ServerPacket::Pong => {
      write_u16(&mut buf, 0);
      write_str(&mut buf, VERSION_ID);
      write_u64(&mut buf, VERSION_ITERATION);
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
    ServerPacket::MapData { map_data } => {
      write_u16(&mut buf, 3);
      write_string(&mut buf, map_data);
    }
    ServerPacket::NaviConnected {
      ticket,
      name,
      x,
      y,
      z,
      warp_in,
    } => {
      write_u16(&mut buf, 4);
      write_string(&mut buf, ticket);
      write_string(&mut buf, name);
      write_f32(&mut buf, f32::floor(x * TILE_WIDTH / 2.0));
      write_f32(&mut buf, f32::floor(y * TILE_HEIGHT));
      write_f32(&mut buf, *z);
      write_bool(&mut buf, *warp_in);
    }
    ServerPacket::NaviDisconnected { ticket } => {
      write_u16(&mut buf, 5);
      write_string(&mut buf, ticket);
    }
    ServerPacket::NaviSetName { ticket, name } => {
      write_u16(&mut buf, 6);
      write_string(&mut buf, ticket);
      write_string(&mut buf, name);
    }
    ServerPacket::NaviMove { ticket, x, y, z } => {
      write_u16(&mut buf, 7);
      write_string(&mut buf, ticket);
      write_f32(&mut buf, f32::floor(x * TILE_WIDTH / 2.0));
      write_f32(&mut buf, f32::floor(y * TILE_HEIGHT));
      write_f32(&mut buf, *z);
    }
    ServerPacket::NaviSetAvatar { ticket, avatar_id } => {
      write_u16(&mut buf, 8);
      write_u16(&mut buf, *avatar_id);
      write_string(&mut buf, ticket);
    }
    ServerPacket::NaviEmote { ticket, emote_id } => {
      write_u16(&mut buf, 9);
      buf.push(*emote_id);
      write_string(&mut buf, ticket);
    }
  }

  buf
}
