use crate::packets::parse_client_packet;
use crate::threads::ThreadMessage;
use std::net::UdpSocket;
use std::sync::mpsc;

const MAX_BUFFER_LEN: usize = 10240;

pub fn create_socket_thread(tx: mpsc::Sender<ThreadMessage>, socket: UdpSocket, log_packets: bool) {
  std::thread::spawn(move || loop {
    let mut buf = [0; MAX_BUFFER_LEN];

    let wrapped_packet = socket.recv_from(&mut buf);
    println!("still running");

    match wrapped_packet {
      Ok((number_of_bytes, src_addr)) => {
        let filled_buf = &mut buf[..number_of_bytes];

        if log_packets {
          println!("Received packet from {}", src_addr);
        }

        let wrapped_packet = parse_client_packet(&filled_buf);

        if let Some(client_packet) = wrapped_packet {
          tx.send(ThreadMessage::ClientPacket(src_addr, client_packet))
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
