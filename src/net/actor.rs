use super::Direction;
use crate::packets::ServerPacket;
use std::time::Instant;

pub struct Actor {
  pub id: String,
  pub name: String,
  pub area_id: String,
  pub texture_path: String,
  pub animation_path: String,
  pub mugshot_texture_path: String,
  pub mugshot_animation_path: String,
  pub direction: Direction,
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub last_movement_time: Instant,
  pub scale_x: f32,
  pub scale_y: f32,
  pub rotation: f32,
  pub minimap_color: (u8, u8, u8, u8),
  pub current_animation: Option<String>,
  pub solid: bool,
}

impl Actor {
  pub fn create_spawn_packet(&self, x: f32, y: f32, z: f32, warp_in: bool) -> ServerPacket {
    ServerPacket::ActorConnected {
      ticket: &self.id,
      name: &self.name,
      texture_path: &self.texture_path,
      animation_path: &self.animation_path,
      direction: self.direction,
      x,
      y,
      z,
      warp_in,
      solid: self.solid,
      scale_x: self.scale_x,
      scale_y: self.scale_y,
      rotation: self.rotation,
      minimap_color: self.minimap_color,
      animation: self.current_animation.as_deref(),
    }
  }

  /// helper function that updates last_movement_time and current_animation if anything has changed
  pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
    #[allow(clippy::float_cmp)]
    let position_changed = self.x != x || self.y != y || self.z != z;

    if !position_changed {
      return;
    }

    self.x = x;
    self.y = y;
    self.z = z;
    self.current_animation = None;
    self.last_movement_time = Instant::now();
  }

  /// helper function that updates last_movement_time if anything has changed
  pub fn set_direction(&mut self, direction: Direction) {
    if self.direction == direction {
      return;
    }

    self.direction = direction;
    self.last_movement_time = Instant::now();
  }
}
