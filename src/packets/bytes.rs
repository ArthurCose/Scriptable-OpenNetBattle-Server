// readers

pub fn read_byte(buf: &mut &[u8]) -> Option<u8> {
  if buf.len() == 0 {
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

pub fn read_f64(buf: &mut &[u8]) -> Option<f64> {
  use byteorder::{ByteOrder, LittleEndian};

  if buf.len() < 8 {
    *buf = &buf[buf.len()..];
    return None;
  }

  let float = LittleEndian::read_f64(buf);

  *buf = &buf[8..];

  Some(float)
}

pub fn read_string(buf: &mut &[u8]) -> Option<String> {
  let terminator_pos = buf.iter().position(|&b| b == 0);

  terminator_pos.and_then(|index| {
    let string_slice = std::str::from_utf8(&buf[0..index]).unwrap();

    *buf = &buf[index + 1..];

    Some(String::from(string_slice))
  })
}

// writers

pub fn write_u16(buf: &mut Vec<u8>, data: u16) {
  use byteorder::{ByteOrder, LittleEndian};

  let mut buf_64 = [0u8; 2];
  LittleEndian::write_u16(&mut buf_64, data);
  buf.extend(&buf_64);
}

pub fn write_u64(buf: &mut Vec<u8>, data: u64) {
  use byteorder::{ByteOrder, LittleEndian};

  let mut buf_64 = [0u8; 8];
  LittleEndian::write_u64(&mut buf_64, data);
  buf.extend(&buf_64);
}

pub fn write_f64(buf: &mut Vec<u8>, data: f64) {
  use byteorder::{ByteOrder, LittleEndian};

  let mut buf_64 = [0u8; 8];
  LittleEndian::write_f64(&mut buf_64, data);
  buf.extend(&buf_64);
}
