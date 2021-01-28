pub struct Player {
  pub socket_address: std::net::SocketAddr,
  pub id: String,
  pub area_id: String,
  pub avatar_id: u16,
  pub x: f64,
  pub y: f64,
  pub z: f64,
  pub ready: bool,
}
