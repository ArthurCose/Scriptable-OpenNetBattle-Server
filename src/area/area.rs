use super::bot::Bot;
use super::map::Map;
use crate::packets::{build_packet, ServerPacket};
use crate::player::Player;
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Area {
  socket: Rc<UdpSocket>,
  map: Map,
  players: HashMap<String, Player>,
  bots: HashMap<String, Bot>,
}

impl Area {
  pub fn new(socket: Rc<UdpSocket>, map: Map) -> Area {
    Area {
      socket,
      map,
      players: HashMap::new(),
      bots: HashMap::new(),
    }
  }

  pub fn get_map(&mut self) -> &mut Map {
    &mut self.map
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

      self.broadcast(&buf);
    }
  }

  pub fn set_player_avatar(&mut self, id: &String, avatar_id: u16) {
    if let Some(player) = self.players.get_mut(id) {
      player.avatar_id = avatar_id;

      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        avatar_id,
      });

      self.broadcast(&buf);
    }
  }

  pub fn set_player_emote(&mut self, id: &String, emote_id: u8) {
    if self.players.contains_key(id) {
      let buf = build_packet(ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      });

      self.broadcast(&buf);
    }
  }

  pub(crate) fn add_player(&mut self, player: Player) {
    // player join packet
    let buf = build_packet(ServerPacket::NaviConnected {
      ticket: player.id.clone(),
    });

    self.broadcast(&buf);

    self.players.insert(player.id.clone(), player);
  }

  pub(crate) fn mark_player_ready(&mut self, id: &String) {
    if let Some(player) = self.players.get_mut(id) {
      player.ready = true;
    }
  }

  pub fn remove_player(&mut self, id: &String) {
    if let Some(_) = self.players.remove(id) {
      let buf = build_packet(ServerPacket::NaviDisconnected { ticket: id.clone() });

      self.broadcast(&buf);
    }
  }

  pub fn add_bot(&mut self, bot: Bot) {
    let buf = build_packet(ServerPacket::NaviConnected {
      ticket: bot.id.clone(),
    });

    self.broadcast(&buf);

    self.bots.insert(bot.id.clone(), bot);
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

      self.broadcast(&buf);
    }
  }

  pub fn set_bot_avatar(&mut self, id: &String, avatar_id: u16) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.avatar_id = avatar_id;

      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        avatar_id,
      });

      self.broadcast(&buf);
    }
  }

  pub fn set_bot_emote(&mut self, id: &String, emote_id: u8) {
    if self.bots.contains_key(id) {
      let buf = build_packet(ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      });

      self.broadcast(&buf);
    }
  }

  pub fn remove_bot(&mut self, id: &String) {
    if let Some(_) = self.bots.remove(id) {
      let buf = build_packet(ServerPacket::NaviDisconnected { ticket: id.clone() });

      self.broadcast(&buf);
    }
  }

  pub(crate) fn broadcast_map_changes(&mut self) {
    if self.map.is_dirty() {
      let buf = build_packet(ServerPacket::MapData {
        map_data: self.map.render(),
      });

      self.broadcast(&buf);
    }
  }

  fn broadcast(&self, buf: &[u8]) {
    for player in self.players.values() {
      if player.ready {
        if let Err(err) = self.socket.send_to(buf, player.socket_address) {
          println!("{:#}", err);
        }
      }
    }
  }
}
