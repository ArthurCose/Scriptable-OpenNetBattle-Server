use crate::packets::{PacketShipper, Reliability, ServerPacket};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct PacketOrchestrator {
  socket: Rc<std::net::UdpSocket>,
  resend_budget: usize,
  client_room_map: HashMap<std::net::SocketAddr, Vec<String>>,
  shipper_map: HashMap<std::net::SocketAddr, Rc<RefCell<PacketShipper>>>,
  rooms: HashMap<String, Vec<Rc<RefCell<PacketShipper>>>>,
  client_id_map: HashMap<String, Rc<RefCell<PacketShipper>>>,
}

impl PacketOrchestrator {
  pub fn new(socket: Rc<std::net::UdpSocket>, resend_budget: usize) -> PacketOrchestrator {
    PacketOrchestrator {
      socket,
      resend_budget,
      client_room_map: HashMap::new(),
      shipper_map: HashMap::new(),
      rooms: HashMap::new(),
      client_id_map: HashMap::new(),
    }
  }

  pub fn add_client(&mut self, socket_address: std::net::SocketAddr, id: String) {
    let shipper = Rc::new(RefCell::new(PacketShipper::new(
      socket_address,
      self.resend_budget,
    )));

    self.client_id_map.insert(id, shipper.clone());

    self.shipper_map.insert(socket_address, shipper);

    self.client_room_map.insert(socket_address, Vec::new());
  }

  pub fn drop_client(&mut self, socket_address: std::net::SocketAddr) {
    // must leave rooms before dropping anything
    if let Some(joined_rooms) = self.client_room_map.get(&socket_address) {
      for room_id in joined_rooms.clone() {
        self.leave_room(socket_address, &room_id);
      }
    }

    self.client_room_map.remove(&socket_address);
    self.shipper_map.remove(&socket_address);
  }

  pub fn join_room(&mut self, socket_address: std::net::SocketAddr, room_id: String) {
    let shipper = if let Some(shipper) = self.shipper_map.get(&socket_address) {
      shipper.clone()
    } else {
      return;
    };

    let joined_rooms = self.client_room_map.get_mut(&socket_address).unwrap();

    if joined_rooms.contains(&room_id) {
      // already in this room
      return;
    }

    let room = if let Some(room) = self.rooms.get_mut(&room_id) {
      room
    } else {
      // need to create the room
      self.rooms.insert(room_id.to_string(), Vec::new());
      self.rooms.get_mut(&room_id).unwrap()
    };

    room.push(shipper);
    joined_rooms.push(room_id.to_string())
  }

  pub fn leave_room(&mut self, socket_address: std::net::SocketAddr, room_id: &str) {
    let shipper = if let Some(shipper) = self.shipper_map.get(&socket_address) {
      shipper
    } else {
      // shipper shouldn't be in any rooms anyway if it doesn't exist
      return;
    };

    let joined_rooms = self.client_room_map.get_mut(&socket_address).unwrap();

    if let Some(index) = joined_rooms.iter().position(|id| room_id == id) {
      // drop tracking
      joined_rooms.remove(index);
    } else {
      // not in this room anyway
      return;
    }

    let room = if let Some(room) = self.rooms.get_mut(room_id) {
      room
    } else {
      // room doesn't exist
      return;
    };

    if let Some(index) = room.iter().position(|s| Rc::ptr_eq(shipper, s)) {
      room.remove(index);
    }

    if room.is_empty() {
      // delete empty room
      self.rooms.remove(room_id);
    }
  }

  pub fn send(
    &mut self,
    socket_address: std::net::SocketAddr,
    reliability: Reliability,
    packet: ServerPacket,
  ) {
    if let Some(shipper) = self.shipper_map.get_mut(&socket_address) {
      shipper.borrow_mut().send(&self.socket, reliability, packet)
    }
  }

  pub fn send_packets(
    &mut self,
    socket_address: std::net::SocketAddr,
    reliability: Reliability,
    packets: Vec<ServerPacket>,
  ) {
    if let Some(shipper) = self.shipper_map.get_mut(&socket_address) {
      let mut shipper = shipper.borrow_mut();

      for packet in packets {
        shipper.send(&self.socket, reliability, packet)
      }
    }
  }

  pub fn send_byte_packets(
    &mut self,
    socket_address: std::net::SocketAddr,
    reliability: Reliability,
    packets: &[Vec<u8>],
  ) {
    if let Some(shipper) = self.shipper_map.get_mut(&socket_address) {
      let mut shipper = shipper.borrow_mut();

      for bytes in packets {
        shipper.send_bytes(&self.socket, reliability, bytes)
      }
    }
  }

  pub fn send_by_id(&mut self, id: &str, reliability: Reliability, packet: ServerPacket) {
    if let Some(shipper) = self.client_id_map.get_mut(id) {
      shipper.borrow_mut().send(&self.socket, reliability, packet)
    }
  }

  #[allow(dead_code)]
  pub fn send_packets_by_id(
    &mut self,
    id: &str,
    reliability: Reliability,
    packets: Vec<ServerPacket>,
  ) {
    if let Some(shipper) = self.client_id_map.get_mut(id) {
      let mut shipper = shipper.borrow_mut();

      for packet in packets {
        shipper.send(&self.socket, reliability, packet)
      }
    }
  }

  pub fn send_byte_packets_by_id(
    &mut self,
    id: &str,
    reliability: Reliability,
    packets: &[Vec<u8>],
  ) {
    if let Some(shipper) = self.client_id_map.get_mut(id) {
      let mut shipper = shipper.borrow_mut();

      for bytes in packets {
        shipper.send_bytes(&self.socket, reliability, bytes)
      }
    }
  }

  pub fn broadcast_to_room(
    &mut self,
    room_id: &str,
    reliability: Reliability,
    packet: ServerPacket,
  ) {
    let room = if let Some(room) = self.rooms.get_mut(room_id) {
      room
    } else {
      return;
    };

    use crate::packets::build_packet;

    let bytes = build_packet(packet);

    for shipper in room {
      shipper
        .borrow_mut()
        .send_bytes(&self.socket, reliability, &bytes)
    }
  }

  pub fn broadcast_bytes_to_room(
    &mut self,
    room_id: &str,
    reliability: Reliability,
    bytes: Vec<u8>,
  ) {
    let room = if let Some(room) = self.rooms.get_mut(room_id) {
      room
    } else {
      return;
    };

    for shipper in room {
      shipper
        .borrow_mut()
        .send_bytes(&self.socket, reliability, &bytes)
    }
  }

  #[allow(dead_code)]
  pub fn broadcast_packets_to_room(
    &mut self,
    room_id: &str,
    reliability: Reliability,
    packets: Vec<ServerPacket>,
  ) {
    use crate::packets::build_packet;

    let byte_packets: Vec<Vec<u8>> = packets.into_iter().map(build_packet).collect();

    self.broadcast_byte_packets_to_room(room_id, reliability, &byte_packets)
  }

  pub fn broadcast_byte_packets_to_room(
    &mut self,
    room_id: &str,
    reliability: Reliability,
    packets: &[Vec<u8>],
  ) {
    let room = if let Some(room) = self.rooms.get_mut(room_id) {
      room
    } else {
      return;
    };

    for shipper in room {
      let mut shipper = shipper.borrow_mut();

      for bytes in packets {
        shipper.send_bytes(&self.socket, reliability, bytes)
      }
    }
  }

  pub fn broadcast(&mut self, reliability: Reliability, packet: ServerPacket) {
    use crate::packets::build_packet;

    let bytes = build_packet(packet);

    for shipper in self.shipper_map.values_mut() {
      shipper
        .borrow_mut()
        .send_bytes(&self.socket, reliability, &bytes);
    }
  }

  pub fn acknowledged(
    &mut self,
    socket_address: std::net::SocketAddr,
    reliability: Reliability,
    id: u64,
  ) {
    if let Some(shipper) = self.shipper_map.get_mut(&socket_address) {
      shipper.borrow_mut().acknowledged(reliability, id)
    }
  }

  pub fn resend_backed_up_packets(&mut self) {
    for shipper in self.shipper_map.values_mut() {
      shipper.borrow_mut().resend_backed_up_packets(&self.socket);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::net::{SocketAddr, UdpSocket};

  fn create_orchestrator() -> PacketOrchestrator {
    let socket = UdpSocket::bind("127.0.0.1:8765").unwrap();
    socket.take_error().unwrap();
    PacketOrchestrator::new(Rc::new(socket), 0)
  }

  #[test]
  fn rooms() {
    let mut orchestrator = create_orchestrator();
    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();

    let room_a = String::from("A");
    let room_b = String::from("B");
    let room_c = String::from("C");

    orchestrator.join_room(addr, room_c.clone());
    orchestrator.add_client(addr, String::new());
    orchestrator.join_room(addr, room_a.clone());
    orchestrator.join_room(addr, room_b.clone());

    assert!(
      orchestrator.shipper_map.contains_key(&addr),
      "shipper should exist"
    );

    assert_eq!(
      orchestrator.client_room_map.get(&addr),
      Some(&vec![room_a.clone(), room_b.clone()]),
      "client should only be in room A and B",
    );

    orchestrator.join_room(addr, room_c.clone());
    orchestrator.leave_room(addr, &room_b);

    assert_eq!(
      orchestrator.client_room_map.get(&addr),
      Some(&vec![room_a.clone(), room_c.clone()]),
      "client should no longer be in room B and should be added to room C"
    );

    assert!(
      orchestrator.rooms.contains_key(&room_a),
      "room A should exist"
    );

    assert!(
      !orchestrator.rooms.contains_key(&room_b),
      "room B should not exist"
    );

    assert!(
      orchestrator.rooms.contains_key(&room_c),
      "room C should exist"
    );

    orchestrator.drop_client(addr);

    assert!(
      !orchestrator.shipper_map.contains_key(&addr),
      "shipper should not exist"
    );

    assert!(
      !orchestrator.client_room_map.contains_key(&addr),
      "joined_rooms list should not exist"
    );

    assert!(
      !orchestrator.rooms.contains_key(&room_a),
      "room A should not exist"
    );

    assert!(
      !orchestrator.rooms.contains_key(&room_c),
      "room C should not exist"
    );
  }
}
