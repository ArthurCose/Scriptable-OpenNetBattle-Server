use crate::area::Area;

pub trait PluginInterface {
  fn init(&mut self, area: &mut Area);
  fn tick(&mut self, area: &mut Area, delta_time: f64);
  fn handle_player_join(&mut self, area: &mut Area, player_id: &String);
  fn handle_player_disconnect(&mut self, area: &mut Area, player_id: &String);
  fn handle_player_move(&mut self, area: &mut Area, player_id: &String, x: f64, y: f64, z: f64); // todo: add a bool return value to prevent default?
  fn handle_player_avatar_change(&mut self, area: &mut Area, player_id: &String, avatar_id: u16);
  fn handle_player_emote(&mut self, area: &mut Area, player_id: &String, emote_id: u8);
}
