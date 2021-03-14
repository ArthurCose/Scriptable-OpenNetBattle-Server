// a boot to kick people with
#[derive(Clone)]
pub struct Boot {
  pub socket_address: std::net::SocketAddr,
  pub reason: String,
}
