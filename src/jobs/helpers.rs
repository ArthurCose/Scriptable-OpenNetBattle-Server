pub async fn resolve_socket_addr(address: &str, port: u16) -> Option<std::net::SocketAddr> {
  use async_std::net::ToSocketAddrs;
  let address_port_pair = (address, port);
  let mut socket_addrs = address_port_pair.to_socket_addrs().await.ok()?;

  socket_addrs.next()
}
