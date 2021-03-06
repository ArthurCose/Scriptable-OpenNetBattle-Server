use crate::packets::parse_client_packet;
use crate::threads::ThreadMessage;
use std::net::UdpSocket;
use std::sync::mpsc;

pub fn create_socket_thread(
  tx: mpsc::Sender<ThreadMessage>,
  socket: UdpSocket,
  max_payload_size: usize,
  log_packets: bool,
) {
  socket
    .set_nonblocking(false)
    .expect("Couldn't set the socket to blocking for the listening thread");

  std::thread::spawn(move || loop {
    let mut buf = vec![0; max_payload_size];

    let wrapped_packet = socket.recv_from(&mut buf);

    match wrapped_packet {
      Ok((number_of_bytes, src_addr)) => {
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
      Err(err) => match err.kind() {
        std::io::ErrorKind::WouldBlock => (),
        _ => panic!("{}", err),
      },
    }
  });
}
