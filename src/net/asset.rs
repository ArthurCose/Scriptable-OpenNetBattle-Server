#[derive(Clone, Debug)]
pub enum Asset {
  Text(String),
  Texture(Vec<u8>),
  Audio(Vec<u8>),
  SFMLImage(Vec<u8>),
}

pub fn get_player_texture_path(player_id: &String) -> String {
  String::from("server/navis/") + player_id + ".texture"
}

pub fn get_player_animation_path(player_id: &String) -> String {
  String::from("server/navis/") + player_id + ".texture"
}

pub fn get_map_path(map_id: &String) -> String {
  String::from("server/maps/") + map_id + ".txt"
}
