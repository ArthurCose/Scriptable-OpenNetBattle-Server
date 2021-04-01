use crate::net::Net;

pub trait PluginInterface {
  fn init(&mut self, net: &mut Net);
  fn tick(&mut self, net: &mut Net, delta_time: f32);
  fn handle_player_request(&mut self, net: &mut Net, player_id: &str, data: &str);
  fn handle_player_connect(&mut self, net: &mut Net, player_id: &str);
  fn handle_player_join(&mut self, net: &mut Net, player_id: &str);
  fn handle_player_transfer(&mut self, net: &mut Net, player_id: &str);
  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &str);
  fn handle_player_move(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32);
  fn handle_player_avatar_change(
    &mut self,
    net: &mut Net,
    player_id: &str,
    texture_path: &str,
    animation_path: &str,
  ) -> bool;
  fn handle_player_emote(&mut self, net: &mut Net, player_id: &str, emote_id: u8) -> bool;
  fn handle_object_interaction(&mut self, net: &mut Net, player_id: &str, tile_object_id: u32);
  fn handle_actor_interaction(&mut self, net: &mut Net, player_id: &str, actor_id: &str);
  fn handle_tile_interaction(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32);
  fn handle_dialog_response(&mut self, net: &mut Net, player_id: &str, response: u8);
}
