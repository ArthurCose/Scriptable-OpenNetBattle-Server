use super::map::Map;

pub struct Area {
  id: String,
  map: Map,
  required_assets: Vec<String>,
  // cache
  connected_players: Vec<String>,
  connected_bots: Vec<String>,
}

impl Area {
  pub fn new(id: String, map: Map) -> Area {
    Area {
      id,
      map,
      required_assets: Vec::new(),
      connected_players: Vec::new(),
      connected_bots: Vec::new(),
    }
  }

  pub fn get_id(&self) -> &str {
    &self.id
  }

  pub fn set_map(&mut self, map: Map) {
    self.map = map;
  }

  pub fn get_map(&self) -> &Map {
    &self.map
  }

  pub fn get_map_mut(&mut self) -> &mut Map {
    &mut self.map
  }

  pub fn require_asset(&mut self, asset_path: String) {
    if !self.required_assets.contains(&asset_path) {
      self.required_assets.push(asset_path);
    }
  }

  pub fn get_required_assets(&self) -> &Vec<String> {
    &self.required_assets
  }

  pub fn get_connected_players(&self) -> &Vec<String> {
    &self.connected_players
  }

  pub(super) fn add_player(&mut self, player_id: String) {
    self.connected_players.push(player_id);
  }

  pub(super) fn remove_player(&mut self, player_id: &str) {
    self
      .connected_players
      .iter()
      .position(|id| id == player_id)
      .map(|position| self.connected_players.swap_remove(position));
  }

  pub fn get_connected_bots(&self) -> &Vec<String> {
    &self.connected_bots
  }

  pub(super) fn add_bot(&mut self, bot_id: String) {
    self.connected_bots.push(bot_id);
  }

  pub(super) fn remove_bot(&mut self, bot_id: &str) {
    self
      .connected_bots
      .iter()
      .position(|id| id == bot_id)
      .map(|position| self.connected_bots.swap_remove(position));
  }
}
