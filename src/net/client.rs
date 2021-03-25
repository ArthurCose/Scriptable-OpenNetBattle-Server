use super::{Direction, Navi};
use crate::packets::PacketShipper;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::net::SocketAddr;

pub(super) struct Client {
  pub socket_address: SocketAddr,
  pub packet_shipper: PacketShipper,
  pub navi: Navi,
  pub warp_in: bool,
  pub warp_x: f32,
  pub warp_y: f32,
  pub warp_z: f32,
  pub ready: bool,
  pub transferring: bool,
  pub cached_assets: HashSet<String>,
  pub texture_buffer: Vec<u8>,
  pub animation_buffer: Vec<u8>,
  message_queue: VecDeque<usize>, // for tracking what plugin sent the message this response is for
}

impl Client {
  pub(super) fn new(
    socket_address: SocketAddr,
    name: String,
    area_id: String,
    spawn_x: f32,
    spawn_y: f32,
    spawn_z: f32,
    resend_budget: usize,
  ) -> Client {
    use super::asset::{get_player_animation_path, get_player_texture_path};
    use uuid::Uuid;

    let id = Uuid::new_v4().to_string();

    Client {
      socket_address,
      packet_shipper: PacketShipper::new(socket_address, resend_budget),
      navi: Navi {
        id: id.clone(),
        name,
        area_id,
        texture_path: get_player_texture_path(&id),
        animation_path: get_player_animation_path(&id),
        direction: Direction::None,
        x: spawn_x,
        y: spawn_y,
        z: spawn_z,
        solid: false,
      },
      warp_in: true,
      warp_x: spawn_x,
      warp_y: spawn_y,
      warp_z: spawn_z,
      ready: false,
      transferring: false,
      cached_assets: HashSet::new(),
      texture_buffer: Vec::new(),
      animation_buffer: Vec::new(),
      message_queue: VecDeque::new(),
    }
  }

  pub fn is_in_widget(&self) -> bool {
    !self.message_queue.is_empty()
  }

  pub(super) fn track_message(&mut self, owner: usize) {
    self.message_queue.push_back(owner);
  }

  pub(super) fn pop_message(&mut self) -> Option<usize> {
    self.message_queue.pop_back()
  }
}

pub(super) fn find_longest_frame_length(animation_data: &str) -> u32 {
  animation_data
    .lines()
    .map(|line| line.trim())
    .filter(|line| line.starts_with("frame"))
    .fold(0, |longest_length, line| {
      let width: i32 = value_of(line, "w").unwrap_or_default();
      let width = width.wrapping_abs() as u32;

      if width > longest_length {
        return width;
      }

      let height: i32 = value_of(line, "h").unwrap_or_default();
      let height = height.wrapping_abs() as u32;

      if height > longest_length {
        return height;
      }

      longest_length
    })
}

fn value_of<T>(line: &str, key: &str) -> Option<T>
where
  T: std::str::FromStr,
{
  let key_index = line.find(key)?;

  // based on ValueOf in bnAnimation.cpp
  // skips the = and ", but technically could be any two values here
  let value_start_index = key_index + key.len() + 2;

  if value_start_index >= line.len() {
    return None;
  }

  let value_slice = &line[value_start_index..];

  let value_end_index = value_slice.find('"')?;
  let value_slice = &value_slice[..value_end_index];

  value_slice.parse().ok()
}
