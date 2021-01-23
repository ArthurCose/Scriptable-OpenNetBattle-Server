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
}

impl Area {
  pub fn new(socket: Rc<UdpSocket>, map: Map) -> Area {
    Area {
      socket,
      map,
      players: HashMap::new(),
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

  pub fn move_player(&mut self, id: &String, x: f64, y: f64, z: f64) -> std::io::Result<()> {
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

      self.broadcast(&buf)?;
    }

    Ok(())
  }

  pub fn set_player_avatar(&mut self, id: &String, avatar_id: u16) -> std::io::Result<()> {
    if let Some(player) = self.players.get_mut(id) {
      player.avatar_id = avatar_id;

      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        avatar_id,
      });

      self.broadcast(&buf)?;
    }

    Ok(())
  }

  pub fn set_player_emote(&mut self, id: &String, emote_id: u8) -> std::io::Result<()> {
    if self.players.contains_key(id) {
      let buf = build_packet(ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      });

      self.broadcast(&buf)?;
    }

    Ok(())
  }

  pub(crate) fn add_player(&mut self, player: Player) -> std::io::Result<()> {
    // player join packet
    let buf = build_packet(ServerPacket::NaviConnected {
      ticket: player.ticket.clone(),
    });

    self.broadcast(&buf)?;

    self.players.insert(player.ticket.clone(), player);

    Ok(())
  }

  pub(crate) fn mark_player_ready(&mut self, id: &String) {
    if let Some(player) = self.players.get_mut(id) {
      player.ready = true;
    }
  }

  pub fn remove_player(&mut self, id: &String) -> std::io::Result<()> {
    if let Some(_) = self.players.remove(id) {
      let buf = build_packet(ServerPacket::NaviDisconnected { ticket: id.clone() });

      self.broadcast(&buf)?;
    }

    Ok(())
  }

  pub(crate) fn broadcast_map_changes(&mut self) -> std::io::Result<()> {
    if self.map.is_dirty() {
      let buf = build_packet(ServerPacket::MapData {
        map_data: self.map.render(),
      });

      self.broadcast(&buf)?;
    }

    Ok(())
  }

  fn broadcast(&self, buf: &[u8]) -> std::io::Result<()> {
    for player in self.players.values() {
      if player.ready {
        self.socket.send_to(buf, player.socket_address)?;
      }
    }
    Ok(())
  }
}
