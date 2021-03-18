// readers

pub fn read_byte(buf: &mut &[u8]) -> Option<u8> {
  if buf.is_empty() {
    return None;
  }

  let byte = buf[0];

  *buf = &buf[1..];

  Some(byte)
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

pub fn read_string(buf: &mut &[u8]) -> Option<String> {
  let terminator_pos = buf.iter().position(|&b| b == 0);

  if let Some(index) = terminator_pos {
    let string_slice = std::str::from_utf8(&buf[0..index]).ok()?;

    *buf = &buf[index + 1..];

    return Some(String::from(string_slice));
  }

  None
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

pub fn write_str(buf: &mut Vec<u8>, data: &str) {
  buf.extend(data.as_bytes());
  buf.push(0);
}

pub fn write_string(buf: &mut Vec<u8>, data: &str) {
  // todo: endianness may be an issue
  buf.extend(data.as_bytes());
  buf.push(0);
}

pub fn write_data(buf: &mut Vec<u8>, data: &[u8]) {
  write_u16(buf, data.len() as u16);
  buf.extend(data);
}
