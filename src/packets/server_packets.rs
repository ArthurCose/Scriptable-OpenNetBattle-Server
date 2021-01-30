use super::bytes::*;
use super::{TILE_HEIGHT, TILE_WIDTH};

pub enum ServerPacket {
  Pong,
  Ack {
    reliability: u8,
    id: u64,
  },
  Login {
    ticket: String,
    error: u16,
  },
  MapData {
    map_data: String,
  },
  NaviConnected {
    ticket: String,
  },
  NaviDisconnected {
    ticket: String,
  },
  NaviWalkTo {
    ticket: String,
    x: f64,
    y: f64,
    z: f64,
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
    }
    ServerPacket::Ack { reliability, id } => {
      write_u16(&mut buf, 1);
      buf.push(*reliability);
      write_u64(&mut buf, *id);
    }
    ServerPacket::Login { ticket, error } => {
      write_u16(&mut buf, 2);
      write_u16(&mut buf, *error);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::MapData { map_data } => {
      write_u16(&mut buf, 3);
      buf.extend(map_data.as_bytes());
    }
    ServerPacket::NaviConnected { ticket } => {
      write_u16(&mut buf, 4);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::NaviDisconnected { ticket } => {
      write_u16(&mut buf, 5);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::NaviWalkTo { ticket, x, y, z } => {
      write_u16(&mut buf, 6);
      buf.extend(ticket.as_bytes());
      write_f64(&mut buf, f64::floor(x * TILE_WIDTH / 2.0));
      write_f64(&mut buf, f64::floor(y * TILE_HEIGHT));
      write_f64(&mut buf, *z);
    }
    ServerPacket::NaviSetAvatar { ticket, avatar_id } => {
      write_u16(&mut buf, 7);
      write_u16(&mut buf, *avatar_id);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::NaviEmote { ticket, emote_id } => {
      write_u16(&mut buf, 8);
      buf.push(*emote_id);
      buf.extend(ticket.as_bytes());
    }
  }

  buf
}
