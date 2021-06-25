pub struct PlayerData {
  pub health: u32,
  pub max_health: u32,
  pub emotion: u8,
  pub money: u32,
  pub items: Vec<String>,
}

impl PlayerData {
  pub fn new() -> PlayerData {
    PlayerData {
      health: 0,
      max_health: 0,
      emotion: 0,
      money: 0,
      items: Vec::new(),
    }
  }
}
