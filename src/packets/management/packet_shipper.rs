use super::super::bytes::write_u64;
use super::super::server_packets::*;
use super::reliability::Reliability;
use log::*;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

struct BackedUpPacket {
  pub id: u64,
  pub creation_time: std::time::Instant,
  pub send_time: std::time::Instant,
  pub data: Vec<u8>,
}

pub struct PacketShipper {
  socket_address: std::net::SocketAddr,
  resend_budget: isize,
  remaining_budget: isize,
  next_unreliable_sequenced: u64,
  next_reliable: u64,
  next_reliable_ordered: u64,
  backed_up_reliable: Vec<BackedUpPacket>,
  backed_up_reliable_ordered: Vec<BackedUpPacket>,
  retry_delay: Duration,
}

impl PacketShipper {
  pub fn new(socket_address: std::net::SocketAddr, resend_budget: usize) -> PacketShipper {
    PacketShipper {
      socket_address,
      resend_budget: resend_budget as isize,
      remaining_budget: resend_budget as isize,
      next_unreliable_sequenced: 0,
      next_reliable: 0,
      next_reliable_ordered: 0,
      backed_up_reliable: Vec::new(),
      backed_up_reliable_ordered: Vec::new(),
      retry_delay: Duration::from_secs(1),
    }
  }

  pub fn get_destination(&self) -> std::net::SocketAddr {
    self.socket_address
  }

  pub fn send(&mut self, socket: &UdpSocket, reliability: Reliability, packet: ServerPacket) {
    self.send_bytes(socket, reliability, &build_packet(packet));
  }

  pub fn send_bytes(&mut self, socket: &UdpSocket, reliability: Reliability, bytes: &[u8]) {
    match reliability {
      Reliability::Unreliable => {
        let mut data = vec![0];
        data.extend(bytes);

        self.send_with_silenced_errors(socket, &data);
      }
      // ignore old packets
      Reliability::UnreliableSequenced => {
        let mut data = vec![1];
        write_u64(&mut data, self.next_unreliable_sequenced);
        data.extend(bytes);

        self.send_with_silenced_errors(socket, &data);

        self.next_unreliable_sequenced += 1;
      }
      Reliability::Reliable => {
        let mut data = vec![2];
        write_u64(&mut data, self.next_reliable);
        data.extend(bytes);

        let creation_time = Instant::now();
        let send_time = if self.send_with_silenced_errors(socket, &data) {
          creation_time
        } else {
          creation_time - self.retry_delay
        };

        self.backed_up_reliable.push(BackedUpPacket {
          id: self.next_reliable,
          creation_time,
          send_time,
          data,
        });

        self.next_reliable += 1;
      }
      // stalls until packets arrive in order (if client gets packet 0 + 3 + 2, it processes 0, and waits for 1)
      Reliability::ReliableOrdered => {
        let mut data = vec![4];
        write_u64(&mut data, self.next_reliable_ordered);
        data.extend(bytes);

        let creation_time = Instant::now();
        let send_time = if self.send_with_silenced_errors(socket, &data) {
          creation_time
        } else {
          creation_time - self.retry_delay
        };

        self.backed_up_reliable_ordered.push(BackedUpPacket {
          id: self.next_reliable_ordered,
          creation_time,
          send_time,
          data,
        });

        self.next_reliable_ordered += 1;
      }
    }
  }

  pub fn resend_backed_up_packets(&mut self, socket: &UdpSocket) {
    use itertools::Itertools;

    self.remaining_budget = self.resend_budget;

    let reliable_iter = self
      .backed_up_reliable
      .iter_mut()
      .take_while(|backed_up_packet| backed_up_packet.send_time.elapsed() >= self.retry_delay);

    let reliable_ordered_iter = self
      .backed_up_reliable_ordered
      .iter_mut()
      .take_while(|backed_up_packet| backed_up_packet.send_time.elapsed() >= self.retry_delay);

    let current_time = Instant::now();

    for backed_up_packet in reliable_iter.interleave(reliable_ordered_iter) {
      if self.remaining_budget < 0 {
        break;
      }

      backed_up_packet.send_time = current_time;

      let buf = &backed_up_packet.data;

      if socket.send_to(buf, self.socket_address).is_err() {
        // socket buffer is probably full
        break;
      }

      self.remaining_budget -= buf.len() as isize;
    }
  }

  pub fn acknowledged(&mut self, reliability: Reliability, id: u64) {
    let acknowledged_packet = match reliability {
      Reliability::Unreliable | Reliability::UnreliableSequenced => {
        debug!("Client is acknowledging unreliable packets?");
        None
      }
      Reliability::Reliable => self.acknowledged_reliable(id),
      Reliability::ReliableOrdered => self.acknowledged_reliable_ordered(id),
    };

    if let Some(packet) = acknowledged_packet {
      let half_ack_speed = packet.creation_time.elapsed() / 2;

      if half_ack_speed < self.retry_delay {
        self.retry_delay = half_ack_speed;
      }
    }
  }

  fn acknowledged_reliable(&mut self, id: u64) -> Option<BackedUpPacket> {
    self
      .backed_up_reliable
      .iter()
      .position(|backed_up| backed_up.id == id)
      .map(|position| self.backed_up_reliable.remove(position))
  }

  fn acknowledged_reliable_ordered(&mut self, id: u64) -> Option<BackedUpPacket> {
    self
      .backed_up_reliable_ordered
      .iter()
      .position(|backed_up| backed_up.id == id)
      .map(|position| self.backed_up_reliable_ordered.remove(position))
  }

  fn send_with_silenced_errors(&mut self, socket: &UdpSocket, buf: &[u8]) -> bool {
    // packet shipper does not guarantee packets being received, but can retry
    // packet sorter will handle kicking

    if self.remaining_budget < 0 {
      return false;
    }

    self.remaining_budget -= buf.len() as isize;

    socket.send_to(buf, self.socket_address).is_ok()
  }
}
