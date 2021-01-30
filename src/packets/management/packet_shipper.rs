use super::super::bytes::write_u64;
use super::super::server_packets::*;
use super::reliability::Reliability;
use std::net::UdpSocket;

struct BackedUpPacket {
  pub id: u64,
  pub creation_time: std::time::Instant,
  pub data: Vec<u8>,
}

pub struct PacketShipper {
  socket_address: std::net::SocketAddr,
  next_unreliable_sequenced: u64,
  next_reliable: u64,
  next_reliable_ordered: u64,
  backed_up_reliable: Vec<BackedUpPacket>,
  backed_up_reliable_ordered: Vec<BackedUpPacket>,
}

impl PacketShipper {
  pub fn new(socket_address: std::net::SocketAddr) -> PacketShipper {
    PacketShipper {
      socket_address,
      next_unreliable_sequenced: 0,
      next_reliable: 0,
      next_reliable_ordered: 0,
      backed_up_reliable: Vec::new(),
      backed_up_reliable_ordered: Vec::new(),
    }
  }

  pub fn send(
    &mut self,
    socket: &UdpSocket,
    reliability: &Reliability,
    packet: &ServerPacket,
  ) -> std::io::Result<()> {
    match reliability {
      Reliability::Unreliable => {
        let mut data = vec![0];
        data.extend(build_packet(packet));

        socket.send_to(&data, self.socket_address)?;
      }
      // ignore old packets
      Reliability::UnreliableSequenced => {
        let mut data = vec![1];
        write_u64(&mut data, self.next_unreliable_sequenced);
        data.extend(build_packet(packet));

        socket.send_to(&data, self.socket_address)?;

        self.next_unreliable_sequenced += 1;
      }
      Reliability::Reliable => {
        let mut data = vec![2];
        write_u64(&mut data, self.next_reliable);
        data.extend(build_packet(packet));

        socket.send_to(&data, self.socket_address)?;

        self.backed_up_reliable.push(BackedUpPacket {
          id: self.next_reliable,
          creation_time: std::time::Instant::now(),
          data,
        });

        self.next_reliable += 1;
      }
      // stalls until packets arrive in order (if client gets packet 0 + 3 + 2, it processes 0, and waits for 1)
      Reliability::ReliableOrdered => {
        let mut data = vec![4];
        write_u64(&mut data, self.next_reliable_ordered);
        data.extend(build_packet(packet));

        socket.send_to(&data, self.socket_address)?;

        self.backed_up_reliable_ordered.push(BackedUpPacket {
          id: self.next_reliable_ordered,
          creation_time: std::time::Instant::now(),
          data,
        });

        self.next_reliable_ordered += 1;
      }
    }

    Ok(())
  }

  pub fn resend_backed_up_packets(&self, socket: &UdpSocket) -> std::io::Result<()> {
    let retry_delay = std::time::Duration::from_secs_f64(1.0 / 20.0);

    for backed_up_packet in &self.backed_up_reliable {
      if backed_up_packet.creation_time.elapsed() < retry_delay {
        break;
      }

      socket.send_to(&backed_up_packet.data, &self.socket_address)?;
    }

    for backed_up_packet in &self.backed_up_reliable_ordered {
      if backed_up_packet.creation_time.elapsed() < retry_delay {
        break;
      }

      socket.send_to(&backed_up_packet.data, &self.socket_address)?;
    }

    Ok(())
  }

  pub fn acknowledged(&mut self, reliability: Reliability, id: u64) {
    match reliability {
      Reliability::Unreliable | Reliability::UnreliableSequenced => {
        println!("Client is acknowledging unreliable packets?")
      }
      Reliability::Reliable => self.acknowledged_reliable(id),
      Reliability::ReliableOrdered => self.acknowledged_reliable_ordered(id),
    }
  }

  fn acknowledged_reliable(&mut self, id: u64) {
    self
      .backed_up_reliable
      .iter()
      .position(|backed_up| backed_up.id == id)
      .map(|position| self.backed_up_reliable.remove(position));
  }

  fn acknowledged_reliable_ordered(&mut self, id: u64) {
    self
      .backed_up_reliable_ordered
      .iter()
      .position(|backed_up| backed_up.id == id)
      .map(|position| self.backed_up_reliable_ordered.remove(position));
  }
}
