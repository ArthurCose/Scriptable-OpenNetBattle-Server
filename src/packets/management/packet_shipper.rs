use super::super::bytes::write_u64;
use super::super::server_packets::*;
use super::reliability::Reliability;
use crate::threads::clock_thread::TICK_RATE;
use std::net::UdpSocket;

struct BackedUpPacket {
  pub id: u64,
  pub creation_time: std::time::Instant,
  pub data: Vec<u8>,
}

pub struct PacketShipper {
  socket_address: std::net::SocketAddr,
  resend_budget: usize,
  next_unreliable_sequenced: u64,
  next_reliable: u64,
  next_reliable_ordered: u64,
  backed_up_reliable: Vec<BackedUpPacket>,
  backed_up_reliable_ordered: Vec<BackedUpPacket>,
}

impl PacketShipper {
  pub fn new(socket_address: std::net::SocketAddr, resend_budget: usize) -> PacketShipper {
    PacketShipper {
      socket_address,
      resend_budget,
      next_unreliable_sequenced: 0,
      next_reliable: 0,
      next_reliable_ordered: 0,
      backed_up_reliable: Vec::new(),
      backed_up_reliable_ordered: Vec::new(),
    }
  }

  pub fn send(&mut self, socket: &UdpSocket, reliability: &Reliability, packet: &ServerPacket) {
    match reliability {
      Reliability::Unreliable => {
        let mut data = vec![0];
        data.extend(build_packet(packet));

        self.send_with_silenced_errors(socket, &data);
      }
      // ignore old packets
      Reliability::UnreliableSequenced => {
        let mut data = vec![1];
        write_u64(&mut data, self.next_unreliable_sequenced);
        data.extend(build_packet(packet));

        self.send_with_silenced_errors(socket, &data);

        self.next_unreliable_sequenced += 1;
      }
      Reliability::Reliable => {
        let mut data = vec![2];
        write_u64(&mut data, self.next_reliable);
        data.extend(build_packet(packet));

        self.send_with_silenced_errors(socket, &data);

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

        self.send_with_silenced_errors(socket, &data);

        self.backed_up_reliable_ordered.push(BackedUpPacket {
          id: self.next_reliable_ordered,
          creation_time: std::time::Instant::now(),
          data,
        });

        self.next_reliable_ordered += 1;
      }
    }
  }

  pub fn resend_backed_up_packets(&self, socket: &UdpSocket) {
    let retry_delay = std::time::Duration::from_secs_f64(1.0 / TICK_RATE);

    let mut remaining_budget = self.resend_budget as isize;

    use itertools::Itertools;

    let reliable_iter = self
      .backed_up_reliable
      .iter()
      .take_while(|backed_up_packet| backed_up_packet.creation_time.elapsed() >= retry_delay);

    let reliable_ordered_iter = self
      .backed_up_reliable_ordered
      .iter()
      .take_while(|backed_up_packet| backed_up_packet.creation_time.elapsed() >= retry_delay);

    for backed_up_packet in reliable_iter.interleave(reliable_ordered_iter) {
      if remaining_budget < 0 {
        break;
      }

      self.send_with_silenced_errors(socket, &backed_up_packet.data);
      remaining_budget -= backed_up_packet.data.len() as isize;
    }
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

  fn send_with_silenced_errors(&self, socket: &UdpSocket, buf: &[u8]) {
    // packet shipper does not guarantee packets being received, but can retry
    // packet sorter will handle kicking
    let _ = socket.send_to(buf, self.socket_address);
  }
}
