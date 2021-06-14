pub struct PlayerData {
  pub money: u32,
}

impl PlayerData {
  pub fn new() -> PlayerData {
    PlayerData { money: 0 }
  }
}
