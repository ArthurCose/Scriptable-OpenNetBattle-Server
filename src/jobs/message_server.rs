pub fn message_server(socket: std::net::UdpSocket, address: String, port: u16, data: Vec<u8>) {
  async_std::task::spawn(async move {
    use super::helpers::*;
    use crate::packets::bytes::*;

    let socket_addr = if let Some(socket_addr) = resolve_socket_addr(address.as_str(), port).await {
      socket_addr
    } else {
      return;
    };

    let mut data = data;

    let mut packet = vec![0]; // unreliable
    write_u16(&mut packet, 2); // server message
    packet.append(&mut data);

    let _ = socket.send_to(&packet, socket_addr);
  });
}
