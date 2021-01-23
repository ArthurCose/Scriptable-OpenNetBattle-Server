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

fn read_byte(buf: &mut &[u8]) -> Option<u8> {
  if buf.len() == 0 {
    return None;
  }

  let byte = buf[0];

  *buf = &buf[1..];

  Some(byte)
}

fn read_u16(buf: &mut &[u8]) -> Option<u16> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 2 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let data = LittleEndian::read_u16(buf);

  *buf = &buf[2..];

  Some(data)
}

fn read_u64(buf: &mut &[u8]) -> Option<u64> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 8 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let data = LittleEndian::read_u64(buf);

  *buf = &buf[8..];

  Some(data)
}

fn read_f64(buf: &mut &[u8]) -> Option<f64> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 8 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let float = LittleEndian::read_f64(buf);

  *buf = &buf[8..];

  Some(float)
}

fn read_string(buf: &mut &[u8]) -> Option<String> {
  let terminator_pos = buf.iter().position(|&b| b == 0);

  terminator_pos.and_then(|index| {
    let string_slice = std::str::from_utf8(&buf[0..index]).unwrap();

    *buf = &buf[index + 1..];

    Some(String::from(string_slice))
  })
}
