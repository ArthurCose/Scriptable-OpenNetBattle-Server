use super::Direction;

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
  pub solid: bool,
}
