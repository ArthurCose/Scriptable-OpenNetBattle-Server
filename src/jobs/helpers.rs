pub fn resolve_socket_addr(address: &str, port: u16) -> Option<std::net::SocketAddr> {
  use std::net::ToSocketAddrs;
  let address_port_pair = (address, port);
  let mut socket_addrs = address_port_pair.to_socket_addrs().ok()?;

  socket_addrs.next()
}
