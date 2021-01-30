use super::bytes::*;
use super::{TILE_HEIGHT, TILE_WIDTH};

pub enum ServerPacket {
  Pong,
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
  NaviSetAvatar {
    ticket: String,
    avatar_id: u16,
  },
  NaviEmote {
    ticket: String,
    emote_id: u8,
  },
  NaviWalkTo {
    ticket: String,
    x: f64,
    y: f64,
    z: f64,
  },
}

pub fn build_packet(packet: ServerPacket) -> Vec<u8> {
  let mut buf = Vec::new(); // u16 error is ignored

  match packet {
    ServerPacket::Pong => {
      write_u16(&mut buf, 9);
    }
    ServerPacket::Login { ticket, error } => {
      write_u16(&mut buf, 0);
      write_u16(&mut buf, error);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::MapData { map_data } => {
      write_u16(&mut buf, 4);
      buf.extend(map_data.as_bytes());
    }
    ServerPacket::NaviConnected { ticket } => {
      write_u16(&mut buf, 8);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::NaviDisconnected { ticket } => {
      write_u16(&mut buf, 5);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::NaviSetAvatar { ticket, avatar_id } => {
      write_u16(&mut buf, 6);
      write_u16(&mut buf, avatar_id);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::NaviEmote { ticket, emote_id } => {
      write_u16(&mut buf, 7);
      buf.push(emote_id);
      buf.extend(ticket.as_bytes());
    }
    ServerPacket::NaviWalkTo { ticket, x, y, z } => {
      write_u16(&mut buf, 1);
      buf.extend(ticket.as_bytes());
      write_f64(&mut buf, f64::floor(x * TILE_WIDTH / 2.0));
      write_f64(&mut buf, f64::floor(y * TILE_HEIGHT));
      write_f64(&mut buf, z);
    }
  }

  buf
}
