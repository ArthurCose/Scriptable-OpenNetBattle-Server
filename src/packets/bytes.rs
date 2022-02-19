// readers

pub fn read_byte(buf: &mut &[u8]) -> Option<u8> {
  if buf.is_empty() {
    return None;
  }

  let byte = buf[0];

  *buf = &buf[1..];

  Some(byte)
}

pub fn read_bool(buf: &mut &[u8]) -> Option<bool> {
  read_byte(buf).map(|byte| byte != 0)
}

pub fn read_u16(buf: &mut &[u8]) -> Option<u16> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 2 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let data = LittleEndian::read_u16(buf);

  *buf = &buf[2..];

  Some(data)
}

pub fn read_u32(buf: &mut &[u8]) -> Option<u32> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 4 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let data = LittleEndian::read_u32(buf);

  *buf = &buf[4..];

  Some(data)
}

pub fn read_u64(buf: &mut &[u8]) -> Option<u64> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 8 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let data = LittleEndian::read_u64(buf);

  *buf = &buf[8..];

  Some(data)
}

pub fn read_f32(buf: &mut &[u8]) -> Option<f32> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 4 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let float = LittleEndian::read_f32(buf);

  *buf = &buf[4..];

  Some(float)
}

pub fn read_string_u8(buf: &mut &[u8]) -> Option<String> {
  let len = read_byte(buf)? as usize;
  read_string(buf, len)
}

pub fn read_string_u16(buf: &mut &[u8]) -> Option<String> {
  let len = read_u16(buf)? as usize;
  read_string(buf, len)
}

fn read_string(buf: &mut &[u8], len: usize) -> Option<String> {
  if buf.len() < len {
    *buf = &buf[buf.len()..];
    return None;
  }

  let string = String::from_utf8_lossy(&buf[..len]).to_string();

  *buf = &buf[len..];

  Some(string)
}

pub fn read_data(buf: &mut &[u8], size: usize) -> Option<Vec<u8>> {
  if buf.len() < size {
    return None;
  }

  let data = Vec::from(&buf[..size]);

  *buf = &buf[size..];

  Some(data)
}

// writers

pub fn write_bool(buf: &mut Vec<u8>, data: bool) {
  buf.push(if data { 1 } else { 0 });
}

pub fn write_u16(buf: &mut Vec<u8>, data: u16) {
  use byteorder::{ByteOrder, LittleEndian};

  let mut buf_64 = [0u8; 2];
  LittleEndian::write_u16(&mut buf_64, data);
  buf.extend(&buf_64);
}

pub fn write_u32(buf: &mut Vec<u8>, data: u32) {
  use byteorder::{ByteOrder, LittleEndian};

  let mut buf_32 = [0u8; 4];
  LittleEndian::write_u32(&mut buf_32, data);
  buf.extend(&buf_32);
}

pub fn write_u64(buf: &mut Vec<u8>, data: u64) {
  use byteorder::{ByteOrder, LittleEndian};

  let mut buf_64 = [0u8; 8];
  LittleEndian::write_u64(&mut buf_64, data);
  buf.extend(&buf_64);
}

pub fn write_f32(buf: &mut Vec<u8>, data: f32) {
  use byteorder::{ByteOrder, LittleEndian};

  let mut buf_32 = [0u8; 4];
  LittleEndian::write_f32(&mut buf_32, data);
  buf.extend(&buf_32);
}

pub fn write_string_u8(buf: &mut Vec<u8>, data: &str) {
  let len = if data.len() < u8::MAX.into() {
    data.len() as u8
  } else {
    u8::MAX
  };

  buf.push(len);
  buf.extend(&data.as_bytes()[0..len.into()]);
}

pub fn write_string_u16(buf: &mut Vec<u8>, data: &str) {
  let len = if data.len() < u16::MAX.into() {
    data.len() as u16
  } else {
    u16::MAX
  };

  write_u16(buf, len);
  buf.extend(&data.as_bytes()[0..len.into()]);
}

pub fn write_data(buf: &mut Vec<u8>, data: &[u8]) {
  write_u16(buf, data.len() as u16);
  buf.extend(data);
}
