use super::{Area, Bot, Map, Player};
use crate::packets::{Reliability, ServerPacket};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Net {
  socket: Rc<UdpSocket>,
  areas: HashMap<String, Area>,
  default_area_id: String,
  players: HashMap<String, Player>,
  bots: HashMap<String, Bot>,
}

impl Net {
  pub fn new(socket: Rc<UdpSocket>) -> Net {
    use std::fs::{read_dir, read_to_string};

    let mut areas = HashMap::new();
    let mut default_area_id = None;

    for wrapped_dir_entry in read_dir("./areas").expect("Area folder missing! (./areas)") {
      if let Ok(map_dir_entry) = wrapped_dir_entry {
        let map_path = map_dir_entry.path();

        if let Ok(raw_map) = read_to_string(&map_path) {
          let map = Map::from(String::from(raw_map));

          if map_path.file_name().unwrap() == "default.txt" {
            default_area_id = Some(map.get_name().clone());
          }

          areas.insert(map.get_name().clone(), Area::new(map));
        }
      }
    }

    Net {
      socket,
      default_area_id: default_area_id.expect("No default (default.txt) area data found"),
      areas,
      players: HashMap::new(),
      bots: HashMap::new(),
    }
  }

  pub fn get_default_area_id(&self) -> &String {
    &self.default_area_id
  }

  pub fn get_area(&mut self, area_id: &String) -> Option<&mut Area> {
    self.areas.get_mut(area_id)
  }

  pub fn get_player(&self, id: &String) -> Option<&Player> {
    self.players.get(id)
  }

  pub(super) fn get_player_mut(&mut self, id: &String) -> Option<&mut Player> {
    self.players.get_mut(id)
  }

  pub fn get_players(&self) -> std::collections::hash_map::Values<String, Player> {
    self.players.values()
  }

  pub fn move_player(&mut self, id: &String, x: f32, y: f32, z: f32) {
    if let Some(player) = self.players.get_mut(id) {
      player.x = x;
      player.y = y;
      player.z = z;

      let packet = ServerPacket::NaviMove {
        ticket: id.clone(),
        x,
        y,
        z,
      };

      let area = self.areas.get(&player.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::UnreliableSequenced,
        packet,
      );
    }
  }

  pub fn set_player_avatar(&mut self, id: &String, avatar_id: u16) {
    if let Some(player) = self.players.get_mut(id) {
      player.avatar_id = avatar_id;

      let packet = ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        avatar_id,
      };

      let area = self.areas.get(&player.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::ReliableOrdered,
        packet,
      );
    }
  }

  pub fn set_player_emote(&mut self, id: &String, emote_id: u8) {
    if let Some(player) = self.players.get(id) {
      let packet = ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      };

      let area = self.areas.get(&player.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub(super) fn add_player(&mut self, player: Player) {
    let area = self.areas.get_mut(&player.area_id).unwrap();

    area.add_player(player.id.clone());
    self.players.insert(player.id.clone(), player);
  }

  pub(super) fn mark_player_ready(&mut self, id: &String) {
    if let Some(player) = self.players.get_mut(id) {
      let area = self.areas.get(&player.area_id).unwrap();

      let packet = ServerPacket::NaviConnected {
        ticket: player.id.clone(),
        name: player.name.clone(),
        x: player.x,
        y: player.y,
        z: player.z,
        warp_in: true,
      };

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::ReliableOrdered,
        packet,
      );
    }
  }

  pub fn remove_player(&mut self, id: &String) {
    if let Some(player) = self.players.remove(id) {
      let area = self
        .areas
        .get_mut(&player.area_id)
        .expect("Missing area for removed player");

      area.remove_player(&player.id);

      let packet = ServerPacket::NaviDisconnected { ticket: id.clone() };

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub fn add_bot(&mut self, bot: Bot) {
    if let Some(area) = self.areas.get_mut(&bot.area_id) {
      area.add_bot(bot.id.clone());

      let packet = ServerPacket::NaviConnected {
        ticket: bot.id.clone(),
        name: bot.name.clone(),
        x: bot.x,
        y: bot.y,
        z: bot.z,
        warp_in: true,
      };

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::ReliableOrdered,
        packet,
      );

      self.bots.insert(bot.id.clone(), bot);
    }
  }

  pub fn get_bot(&self, id: &String) -> Option<&Bot> {
    self.bots.get(id)
  }

  pub fn get_bots(&self) -> std::collections::hash_map::Values<String, Bot> {
    self.bots.values()
  }

  pub fn move_bot(&mut self, id: &String, x: f32, y: f32, z: f32) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.x = x;
      bot.y = y;
      bot.z = z;

      let packet = ServerPacket::NaviMove {
        ticket: id.clone(),
        x,
        y,
        z,
      };

      let area = self.areas.get(&bot.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::UnreliableSequenced,
        packet,
      );
    }
  }

  pub fn set_bot_avatar(&mut self, id: &String, avatar_id: u16) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.avatar_id = avatar_id;

      let packet = ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        avatar_id,
      };

      let area = self.areas.get(&bot.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::ReliableOrdered,
        packet,
      );
    }
  }

  pub fn set_bot_emote(&mut self, id: &String, emote_id: u8) {
    if let Some(bot) = self.bots.get(id) {
      let packet = ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      };

      let area = self.areas.get(&bot.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub fn remove_bot(&mut self, id: &String) {
    if let Some(bot) = self.bots.remove(id) {
      let area = self
        .areas
        .get_mut(&bot.area_id)
        .expect("Missing area for removed bot");

      area.remove_bot(&bot.id);

      let packet = ServerPacket::NaviDisconnected { ticket: id.clone() };

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub(super) fn broadcast_map_changes(&mut self) {
    for area in self.areas.values_mut() {
      let map = area.get_map();

      if map.is_dirty() {
        let packet = ServerPacket::MapData {
          map_data: map.render(),
        };

        broadcast_to_area(
          &self.socket,
          &mut self.players,
          area,
          Reliability::ReliableOrdered,
          packet,
        );
      }
    }
  }

  pub(super) fn resend_backed_up_packets(&mut self) -> Vec<std::net::SocketAddr> {
    let mut disconnected_addresses = Vec::new();

    for player in self.players.values_mut() {
      if let Err(_) = player.packet_shipper.resend_backed_up_packets(&self.socket) {
        disconnected_addresses.push(player.socket_address);
      }
    }

    disconnected_addresses
  }
}

fn broadcast_to_area(
  socket: &UdpSocket,
  players: &mut HashMap<String, Player>,
  area: &Area,
  reliability: Reliability,
  packet: ServerPacket,
) {
  for player_id in area.get_connected_players() {
    let player = players.get_mut(player_id).unwrap();

    if let Err(err) = player.packet_shipper.send(socket, &reliability, &packet) {
      println!("{:#}", err);
    }
  }
}
