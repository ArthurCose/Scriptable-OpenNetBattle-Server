use crate::packets::parse_client_packet;
use crate::threads::ThreadMessage;
use std::net::UdpSocket;
use std::sync::mpsc;

pub fn create_listening_thread(
  tx: mpsc::Sender<ThreadMessage>,
  socket: UdpSocket,
  max_payload_size: usize,
  log_packets: bool,
) {
  let async_socket = async_std::net::UdpSocket::from(socket);

  std::thread::spawn(move || {
    async_std::task::block_on(listen_loop(tx, async_socket, max_payload_size, log_packets))
  });
}

async fn listen_loop(
  tx: mpsc::Sender<ThreadMessage>,
  async_socket: async_std::net::UdpSocket,
  max_payload_size: usize,
  log_packets: bool,
) {
  loop {
    let mut buf = vec![0; max_payload_size];

    let wrapped_packet = async_socket.recv_from(&mut buf).await;

    if wrapped_packet.is_err() {
      // don't bring down the whole server over one "connection"
      continue;
    }

    let (number_of_bytes, src_addr) = wrapped_packet.unwrap();
    let filled_buf = &buf[..number_of_bytes];

    if log_packets {
      println!("Received packet from {}", src_addr);
    }

    if let Some((headers, packet)) = parse_client_packet(&filled_buf) {
      tx.send(ThreadMessage::ClientPacket {
        socket_address: src_addr,
        headers,
        packet,
      })
      .unwrap();
    } else {
      println!("Received unknown packet from {}", src_addr);
      println!("{:?}", filled_buf);
    }
  }
}
