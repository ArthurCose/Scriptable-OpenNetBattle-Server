use crate::net::Net;

pub trait PluginInterface {
  fn init(&mut self, net: &mut Net);
  fn tick(&mut self, net: &mut Net, delta_time: f32);
  fn handle_player_connect(&mut self, net: &mut Net, player_id: &str);
  fn handle_player_transfer(&mut self, net: &mut Net, player_id: &str);
  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &str);
  fn handle_player_move(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32); // todo: add a bool return value to prevent default?
  fn handle_player_avatar_change(
    &mut self,
    net: &mut Net,
    player_id: &str,
    texture_path: &str,
    animation_path: &str,
  );
  fn handle_player_emote(&mut self, net: &mut Net, player_id: &str, emote_id: u8);
  fn handle_object_interaction(&mut self, net: &mut Net, player_id: &str, tile_object_id: u32);
  fn handle_navi_interaction(&mut self, net: &mut Net, player_id: &str, navi_id: &str);
  fn handle_tile_interaction(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32);
}
