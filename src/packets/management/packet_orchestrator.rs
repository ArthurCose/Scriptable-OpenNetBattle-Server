use crate::packets::{PacketShipper, Reliability, ServerPacket};
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct PacketOrchestrator {
  socket: Rc<std::net::UdpSocket>,
  resend_budget: usize,
  client_id_map: HashMap<String, Rc<RefCell<PacketShipper>>>,
  client_room_map: HashMap<std::net::SocketAddr, Vec<String>>,
  shipper_map: HashMap<std::net::SocketAddr, Rc<RefCell<PacketShipper>>>,
  rooms: HashMap<String, Vec<Rc<RefCell<PacketShipper>>>>,
  synchronize_updates: bool,
  synchronize_requests: usize,
  synchronize_locked_clients: HashSet<std::net::SocketAddr>,
}

impl PacketOrchestrator {
  pub fn new(socket: Rc<std::net::UdpSocket>, resend_budget: usize) -> PacketOrchestrator {
    PacketOrchestrator {
      socket,
      resend_budget,
      client_id_map: HashMap::new(),
      client_room_map: HashMap::new(),
      shipper_map: HashMap::new(),
      rooms: HashMap::new(),
      synchronize_updates: false,
      synchronize_requests: 0,
      synchronize_locked_clients: HashSet::new(),
    }
  }

  pub fn request_update_synchronization(&mut self) {
    self.synchronize_updates = true;
    self.synchronize_requests += 1;
  }

  pub fn request_disable_update_synchronization(&mut self) {
    if self.synchronize_requests == 0 {
      println!("disable_update_synchronization called too many times!");
      return;
    }

    self.synchronize_requests -= 1;

    if self.synchronize_requests > 0 {
      return;
    }

    use crate::packets::build_packet;

    let bytes = build_packet(ServerPacket::EndSynchronization);

    for socket_address in &self.synchronize_locked_clients {
      if let Some(shipper) = self.shipper_map.get_mut(socket_address) {
        shipper
          .borrow_mut()
          .send_bytes(&self.socket, Reliability::ReliableOrdered, &bytes);
      }
    }

    self.synchronize_locked_clients.clear();
    self.synchronize_updates = false;
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
      internal_send_packet(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        packet,
      )
    }
  }

  pub fn send_packets(
    &mut self,
    socket_address: std::net::SocketAddr,
    reliability: Reliability,
    packets: Vec<ServerPacket>,
  ) {
    if let Some(shipper) = self.shipper_map.get_mut(&socket_address) {
      internal_send_packets(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        packets,
      );
    }
  }

  pub fn send_byte_packets(
    &mut self,
    socket_address: std::net::SocketAddr,
    reliability: Reliability,
    packets: &[Vec<u8>],
  ) {
    if let Some(shipper) = self.shipper_map.get_mut(&socket_address) {
      internal_send_byte_packets(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        packets,
      );
    }
  }

  pub fn send_by_id(&mut self, id: &str, reliability: Reliability, packet: ServerPacket) {
    if let Some(shipper) = self.client_id_map.get_mut(id) {
      internal_send_packet(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        packet,
      );
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
      internal_send_packets(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        packets,
      );
    }
  }

  pub fn send_byte_packets_by_id(
    &mut self,
    id: &str,
    reliability: Reliability,
    packets: &[Vec<u8>],
  ) {
    if let Some(shipper) = self.client_id_map.get_mut(id) {
      internal_send_byte_packets(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        packets,
      );
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
      internal_send_bytes(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        &bytes,
      );
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
      internal_send_bytes(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        &bytes,
      );
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
      internal_send_byte_packets(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        packets,
      );
    }
  }

  pub fn broadcast(&mut self, reliability: Reliability, packet: ServerPacket) {
    use crate::packets::build_packet;

    let bytes = build_packet(packet);

    for shipper in self.shipper_map.values_mut() {
      internal_send_bytes(
        self.synchronize_updates,
        &mut self.synchronize_locked_clients,
        &self.socket,
        shipper,
        reliability,
        &bytes,
      );
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

// funneling sending packets to these internal functions to handle synchronized updates
fn handle_synchronization(
  synchronize_updates: bool,
  synchronize_locked_clients: &mut HashSet<std::net::SocketAddr>,
  socket: &std::net::UdpSocket,
  shipper: &mut RefMut<PacketShipper>,
  reliability: Reliability,
) -> Reliability {
  if !synchronize_updates || synchronize_locked_clients.contains(&shipper.get_destination()) {
    return reliability;
  }

  synchronize_locked_clients.insert(shipper.get_destination());

  shipper.send(
    socket,
    Reliability::ReliableOrdered,
    ServerPacket::SynchronizeUpdates,
  );

  // force reliable ordered for synchronization
  Reliability::ReliableOrdered
}

fn internal_send_packet(
  synchronize_updates: bool,
  synchronize_locked_clients: &mut HashSet<std::net::SocketAddr>,
  socket: &std::net::UdpSocket,
  shipper: &RefCell<PacketShipper>,
  reliability: Reliability,
  packet: ServerPacket,
) {
  let mut shipper = shipper.borrow_mut();

  let reliability = handle_synchronization(
    synchronize_updates,
    synchronize_locked_clients,
    socket,
    &mut shipper,
    reliability,
  );

  shipper.send(socket, reliability, packet);
}

fn internal_send_bytes(
  synchronize_updates: bool,
  synchronize_locked_clients: &mut HashSet<std::net::SocketAddr>,
  socket: &std::net::UdpSocket,
  shipper: &RefCell<PacketShipper>,
  reliability: Reliability,
  bytes: &[u8],
) {
  let mut shipper = shipper.borrow_mut();

  let reliability = handle_synchronization(
    synchronize_updates,
    synchronize_locked_clients,
    socket,
    &mut shipper,
    reliability,
  );

  shipper.send_bytes(socket, reliability, bytes);
}

fn internal_send_packets(
  synchronize_updates: bool,
  synchronize_locked_clients: &mut HashSet<std::net::SocketAddr>,
  socket: &std::net::UdpSocket,
  shipper: &RefCell<PacketShipper>,
  reliability: Reliability,
  packets: Vec<ServerPacket>,
) {
  let mut shipper = shipper.borrow_mut();

  let reliability = handle_synchronization(
    synchronize_updates,
    synchronize_locked_clients,
    socket,
    &mut shipper,
    reliability,
  );

  for packet in packets {
    shipper.send(socket, reliability, packet);
  }
}

fn internal_send_byte_packets(
  synchronize_updates: bool,
  synchronize_locked_clients: &mut HashSet<std::net::SocketAddr>,
  socket: &std::net::UdpSocket,
  shipper: &RefCell<PacketShipper>,
  reliability: Reliability,
  packets: &[Vec<u8>],
) {
  let mut shipper = shipper.borrow_mut();

  let reliability = handle_synchronization(
    synchronize_updates,
    synchronize_locked_clients,
    socket,
    &mut shipper,
    reliability,
  );

  for bytes in packets {
    shipper.send_bytes(socket, reliability, bytes);
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
