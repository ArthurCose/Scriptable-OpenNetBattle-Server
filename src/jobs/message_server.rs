use super::Job;

pub fn message_server(
  socket: std::net::UdpSocket,
  address: String,
  port: u16,
  data: Vec<u8>,
) -> Job {
  Box::new(move || {
    use super::helpers::*;
    use crate::packets::bytes::*;

    let socket_addr = if let Some(socket_addr) = resolve_socket_addr(address.as_str(), port) {
      socket_addr
    } else {
      return;
    };

    let mut data = data;

    let mut packet = Vec::new();
    packet.push(0); // unreliable
    write_u16(&mut packet, 15);
    packet.append(&mut data);

    let _ = socket.send_to(&packet, socket_addr);
  })
}
