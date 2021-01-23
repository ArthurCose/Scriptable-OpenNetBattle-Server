use crate::map::Map;
use std::cell::RefCell;

pub trait Plugin {
  fn tick(&mut self, server: &mut RefCell<Map>, delta_time: f64);
  fn handle_player_join(&mut self, server: &mut RefCell<Map>, ticket: String);
  fn handle_player_disconnect(&mut self, server: &mut RefCell<Map>, ticket: String);
  fn handle_player_move(
    &mut self,
    server: &mut RefCell<Map>,
    ticket: String,
    x: f64,
    y: f64,
    z: f64,
  ); // todo: add a bool return value to prevent default?
  fn handle_player_avatar_change(
    &mut self,
    server: &mut RefCell<Map>,
    ticket: String,
    avatar_id: u16,
  );
  fn handle_player_emote(&mut self, server: &mut RefCell<Map>, ticket: String, emote_id: u8);
}
