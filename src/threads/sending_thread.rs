use std::net::{SocketAddr, UdpSocket};
use std::sync::{mpsc, Arc};

type Message = (SocketAddr, Vec<u8>);

#[derive(Clone)]
pub struct PacketSendingChannel {
  tx: mpsc::Sender<Message>,
}

impl PacketSendingChannel {
  pub fn send_to(&self, addr: SocketAddr, buf: Vec<u8>) {
    self
      .tx
      .send((addr, buf))
      .expect("Packet sending thread closed before sending channels?");
  }
}

// use async std to get blocking reads but non blocking sends
pub fn create_sending_thread(socket: UdpSocket) -> PacketSendingChannel {
  let (tx, rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();
  let async_socket = Arc::new(async_std::net::UdpSocket::from(socket));

  std::thread::spawn(move || async_std::task::block_on(listen_loop(async_socket, rx)));

  PacketSendingChannel { tx }
}

async fn listen_loop(async_socket: Arc<async_std::net::UdpSocket>, rx: mpsc::Receiver<Message>) {
  loop {
    if let Ok((addr, data)) = rx.recv() {
      async_std::task::spawn(create_task(async_socket.clone(), addr, data));
    } else {
      break;
    }
  }
}

async fn create_task(
  async_socket: Arc<async_std::net::UdpSocket>,
  addr: SocketAddr,
  data: Vec<u8>,
) {
  let _ = async_socket.send_to(&data, addr).await;
}
