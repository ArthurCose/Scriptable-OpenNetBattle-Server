use super::{Area, Bot, Map, Player};
use crate::packets::{build_packet, ServerPacket};
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

  pub fn get_players(&self) -> std::collections::hash_map::Values<String, Player> {
    self.players.values()
  }

  pub fn move_player(&mut self, id: &String, x: f64, y: f64, z: f64) {
    if let Some(player) = self.players.get_mut(id) {
      player.x = x;
      player.y = y;
      player.z = z;

      let buf = build_packet(ServerPacket::NaviWalkTo {
        ticket: id.clone(),
        x,
        y,
        z,
      });

      let area = self.areas.get(&player.area_id).unwrap();
      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub fn set_player_avatar(&mut self, id: &String, avatar_id: u16) {
    if let Some(player) = self.players.get_mut(id) {
      player.avatar_id = avatar_id;

      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        avatar_id,
      });

      let area = self.areas.get(&player.area_id).unwrap();
      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub fn set_player_emote(&mut self, id: &String, emote_id: u8) {
    if let Some(player) = self.players.get(id) {
      let buf = build_packet(ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      });

      let area = self.areas.get(&player.area_id).unwrap();
      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub(crate) fn add_player(&mut self, player: Player) {
    let area = self.areas.get_mut(&player.area_id).unwrap();

    area.add_player(player.id.clone());

    let buf = build_packet(ServerPacket::NaviConnected {
      ticket: player.id.clone(),
    });

    self.players.insert(player.id.clone(), player);

    broadcast_to_area(&self.socket, &self.players, area, &buf);
  }

  pub(crate) fn mark_player_ready(&mut self, id: &String) {
    if let Some(player) = self.players.get_mut(id) {
      player.ready = true;
    }
  }

  pub fn remove_player(&mut self, id: &String) {
    if let Some(player) = self.players.remove(id) {
      let area = self
        .areas
        .get_mut(&player.area_id)
        .expect("Missing area for removed player");

      area.remove_player(&player.id);

      let buf = build_packet(ServerPacket::NaviDisconnected { ticket: id.clone() });

      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub fn add_bot(&mut self, bot: Bot) {
    if let Some(area) = self.areas.get_mut(&bot.area_id) {
      area.add_bot(bot.id.clone());

      let buf = build_packet(ServerPacket::NaviConnected {
        ticket: bot.id.clone(),
      });

      broadcast_to_area(&self.socket, &self.players, area, &buf);

      self.bots.insert(bot.id.clone(), bot);
    }
  }

  pub fn get_bot(&self, id: &String) -> Option<&Bot> {
    self.bots.get(id)
  }

  pub fn get_bots(&self) -> std::collections::hash_map::Values<String, Bot> {
    self.bots.values()
  }

  pub fn move_bot(&mut self, id: &String, x: f64, y: f64, z: f64) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.x = x;
      bot.y = y;
      bot.z = z;

      let buf = build_packet(ServerPacket::NaviWalkTo {
        ticket: id.clone(),
        x,
        y,
        z,
      });

      let area = self.areas.get(&bot.area_id).unwrap();
      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub fn set_bot_avatar(&mut self, id: &String, avatar_id: u16) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.avatar_id = avatar_id;

      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        avatar_id,
      });

      let area = self.areas.get(&bot.area_id).unwrap();
      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub fn set_bot_emote(&mut self, id: &String, emote_id: u8) {
    if let Some(bot) = self.bots.get(id) {
      let buf = build_packet(ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      });

      let area = self.areas.get(&bot.area_id).unwrap();
      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub fn remove_bot(&mut self, id: &String) {
    if let Some(bot) = self.bots.remove(id) {
      let area = self
        .areas
        .get_mut(&bot.area_id)
        .expect("Missing area for removed bot");

      area.remove_bot(&bot.id);

      let buf = build_packet(ServerPacket::NaviDisconnected { ticket: id.clone() });

      broadcast_to_area(&self.socket, &self.players, area, &buf);
    }
  }

  pub(crate) fn broadcast_map_changes(&mut self) {
    for area in self.areas.values_mut() {
      let map = area.get_map();

      if map.is_dirty() {
        let buf = build_packet(ServerPacket::MapData {
          map_data: map.render(),
        });

        broadcast_to_area(&self.socket, &self.players, area, &buf);
      }
    }
  }
}

fn broadcast_to_area(
  socket: &UdpSocket,
  players: &HashMap<String, Player>,
  area: &Area,
  buf: &[u8],
) {
  for player_id in area.get_connected_players() {
    let player = players.get(player_id).unwrap();

    if player.ready {
      if let Err(err) = socket.send_to(buf, player.socket_address) {
        println!("{:#}", err);
      }
    }
  }
}
