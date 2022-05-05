use crate::net::ServerConfig;
use crate::packets::parse_client_packet;
use crate::threads::ThreadMessage;
use log::*;
use std::net::UdpSocket;
use std::sync::mpsc;

pub fn create_listening_thread(
  tx: mpsc::Sender<ThreadMessage>,
  socket: UdpSocket,
  config: ServerConfig,
) {
  let async_socket = async_std::net::UdpSocket::from(socket);

  async_std::task::spawn(listen_loop(tx, async_socket, config));
}

async fn listen_loop(
  tx: mpsc::Sender<ThreadMessage>,
  async_socket: async_std::net::UdpSocket,
  config: ServerConfig,
) {
  loop {
    let mut buf = vec![0; config.max_payload_size];

    let wrapped_packet = async_socket.recv_from(&mut buf).await;

    if should_drop(config.receiving_drop_rate) {
      // this must come after the recv_from
      continue;
    }

    if wrapped_packet.is_err() {
      // don't bring down the whole server over one "connection"
      continue;
    }

    let (number_of_bytes, src_addr) = wrapped_packet.unwrap();
    let filled_buf = &buf[..number_of_bytes];

    if config.log_packets {
      debug!("Received packet from {}", src_addr);
    }

    if let Some((headers, packet)) = parse_client_packet(filled_buf) {
      tx.send(ThreadMessage::ClientPacket {
        socket_address: src_addr,
        headers,
        packet,
      })
      .unwrap();
    } else {
      debug!("Received unknown packet from {}", src_addr);
      debug!("{:?}", filled_buf);
    }
  }
}

fn should_drop(rate: f32) -> bool {
  if rate == 0.0 {
    return false;
  }

  let roll: f32 = rand::random();

  roll < rate / 100.0
}
