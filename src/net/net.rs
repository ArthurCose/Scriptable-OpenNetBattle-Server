use super::{Area, Asset, Map, Navi, Player};
use crate::packets::{create_asset_stream, PacketShipper, Reliability, ServerPacket};
use std::collections::{HashMap, HashSet};
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Net {
  socket: Rc<UdpSocket>,
  areas: HashMap<String, Area>,
  default_area_id: String,
  players: HashMap<String, Player>,
  bots: HashMap<String, Navi>,
  assets: HashMap<String, Asset>,
}

impl Net {
  pub fn new(socket: Rc<UdpSocket>) -> Net {
    use super::asset::get_map_path;
    use std::fs::{read_dir, read_to_string};

    let mut assets = HashMap::new();
    Net::load_assets_from_dir(&mut assets, &std::path::Path::new("assets"));

    let mut areas = HashMap::new();
    let mut default_area_id = None;

    for wrapped_dir_entry in read_dir("./areas").expect("Area folder missing! (./areas)") {
      if let Ok(map_dir_entry) = wrapped_dir_entry {
        let map_path = map_dir_entry.path();

        if let Ok(raw_map) = read_to_string(&map_path) {
          let mut map = Map::from(String::from(raw_map));

          if map_path.file_name().unwrap() == "default.txt" {
            default_area_id = Some(map.get_name().clone());
          }

          assets.insert(get_map_path(map.get_name()), Asset::Text(map.render()));
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
      assets,
    }
  }

  fn load_assets_from_dir(assets: &mut HashMap<String, Asset>, dir: &std::path::Path) {
    use std::fs::{read, read_dir, read_to_string};

    if let Ok(entries) = read_dir(dir) {
      for wrapped_entry in entries {
        if let Ok(entry) = wrapped_entry {
          let path = entry.path();

          if path.is_dir() {
            Net::load_assets_from_dir(assets, &path);
          } else {
            let path_string = String::from("server/") + path.to_str().unwrap_or_default();
            let extension_index = path_string.rfind(".").unwrap_or(path_string.len());
            let extension = path_string.to_lowercase().split_off(extension_index);

            let asset = if extension == ".ogg" {
              Asset::Audio(read(&path).unwrap_or_default())
            } else if extension == ".png" || extension == ".bmp" {
              Asset::Texture(read(&path).unwrap_or_default())
            } else {
              Asset::Text(read_to_string(&path).unwrap_or_default())
            };

            assets.insert(path_string, asset);
          }
        }
      }
    }
  }

  pub fn get_default_area_id(&self) -> &String {
    &self.default_area_id
  }

  pub fn get_area(&self, area_id: &String) -> Option<&Area> {
    self.areas.get(area_id)
  }

  pub fn get_area_mut(&mut self, area_id: &String) -> Option<&mut Area> {
    self.areas.get_mut(area_id)
  }

  pub fn get_asset(&self, path: &String) -> Option<&Asset> {
    self.assets.get(path)
  }

  pub fn set_asset(&mut self, path: String, asset: Asset) {
    update_cached_players(&self.socket, &mut self.players, &path, &asset);

    self.assets.insert(path, asset);
  }

  pub fn remove_asset(&mut self, path: &String) {
    self.assets.remove(path);
  }

  pub fn get_player(&self, id: &String) -> Option<&Player> {
    self.players.get(id)
  }

  pub(super) fn get_player_mut(&mut self, id: &String) -> Option<&mut Player> {
    self.players.get_mut(id)
  }

  pub fn set_player_name(&mut self, id: &String, name: String) {
    if let Some(player) = self.players.get_mut(id) {
      player.navi.name = name.clone();

      // skip if player has not even been sent to anyone yet
      if player.ready {
        let packet = ServerPacket::NaviSetName {
          ticket: id.clone(),
          name,
        };

        let area = self.areas.get(&player.navi.area_id).unwrap();

        broadcast_to_area(
          &self.socket,
          &mut self.players,
          area,
          Reliability::Reliable,
          packet,
        );
      }
    }
  }

  pub fn move_player(&mut self, id: &String, x: f32, y: f32, z: f32) {
    if let Some(player) = self.players.get_mut(id) {
      player.navi.x = x;
      player.navi.y = y;
      player.navi.z = z;

      // skip if player has not even been sent to anyone yet
      if player.ready {
        let packet = ServerPacket::NaviMove {
          ticket: id.clone(),
          x,
          y,
          z,
        };

        let area = self.areas.get(&player.navi.area_id).unwrap();

        broadcast_to_area(
          &self.socket,
          &mut self.players,
          area,
          Reliability::UnreliableSequenced,
          packet,
        );
      }
    }
  }

  pub fn set_player_avatar(&mut self, id: &String, texture_path: String, animation_path: String) {
    if let Some(player) = self.players.get_mut(id) {
      player.navi.texture_path = texture_path.clone();
      player.navi.animation_path = animation_path.clone();

      let area = self.areas.get(&player.navi.area_id).unwrap();

      // skip if player has not even been sent to anyone yet
      if player.ready {
        assert_asset(
          &self.socket,
          area,
          &self.assets,
          &mut self.players,
          &texture_path,
        );

        assert_asset(
          &self.socket,
          area,
          &self.assets,
          &mut self.players,
          &animation_path,
        );

        let packet = ServerPacket::NaviSetAvatar {
          ticket: id.clone(),
          texture_path,
          animation_path,
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

  pub fn set_player_emote(&mut self, id: &String, emote_id: u8) {
    if let Some(player) = self.players.get(id) {
      let packet = ServerPacket::NaviEmote {
        ticket: id.clone(),
        emote_id,
      };

      let area = self.areas.get(&player.navi.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.players,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub(super) fn add_player(
    &mut self,
    socket_address: std::net::SocketAddr,
    name: String,
    texture_data: Vec<u8>,
    animation_data: String,
  ) -> std::io::Result<String> {
    use uuid::Uuid;

    let id = Uuid::new_v4().to_string();

    let (texture_path, animation_path) =
      self.store_player_avatar(&id, texture_data, animation_data);

    let area_id = self.get_default_area_id().clone();
    let area = self.get_area_mut(&area_id).unwrap();
    let (spawn_x, spawn_y) = area.get_map().get_spawn();

    let player = Player {
      socket_address,
      packet_shipper: PacketShipper::new(socket_address),
      navi: Navi {
        id: id.clone(),
        name,
        area_id,
        texture_path,
        animation_path,
        x: spawn_x,
        y: spawn_y,
        z: 0.0,
      },
      ready: false,
      cached_assets: HashSet::new(),
    };

    area.add_player(player.navi.id.clone());
    self.players.insert(player.navi.id.clone(), player);

    Ok(id)
  }

  pub(super) fn store_player_avatar(
    &mut self,
    player_id: &String,
    texture_data: Vec<u8>,
    animation_data: String,
  ) -> (String, String) {
    use super::asset;

    let texture_path = asset::get_player_texture_path(player_id);
    let animation_path = asset::get_player_animation_path(player_id);

    self.set_asset(texture_path.clone(), Asset::SFMLImage(texture_data));
    self.set_asset(animation_path.clone(), Asset::Text(animation_data));

    (texture_path, animation_path)
  }

  pub(super) fn connect_player(&mut self, player_id: &String) -> std::io::Result<()> {
    use super::asset::get_map_path;

    let mut packets: Vec<ServerPacket> = Vec::new();
    let mut asset_paths: Vec<String> = Vec::new();

    let player = self.players.get_mut(player_id).unwrap();

    // send map
    let map_path = get_map_path(&player.navi.area_id);
    asset_paths.push(map_path.clone());
    packets.push(ServerPacket::MapUpdate { map_path });

    let area = self.areas.get(&player.navi.area_id).unwrap();

    // send players
    for other_player_id in area.get_connected_players() {
      if other_player_id == player_id {
        continue;
      }

      let other_player = self.players.get(other_player_id).unwrap();

      asset_paths.push(other_player.navi.texture_path.clone());
      asset_paths.push(other_player.navi.animation_path.clone());

      packets.push(ServerPacket::NaviConnected {
        ticket: other_player.navi.id.clone(),
        name: other_player.navi.name.clone(),
        texture_path: other_player.navi.texture_path.clone(),
        animation_path: other_player.navi.animation_path.clone(),
        x: other_player.navi.x,
        y: other_player.navi.y,
        z: other_player.navi.z,
        warp_in: false,
      });
    }

    // send bots
    for bot_id in area.get_connected_bots() {
      let bot = self.bots.get(bot_id).unwrap();

      asset_paths.push(bot.texture_path.clone());
      asset_paths.push(bot.animation_path.clone());

      packets.push(ServerPacket::NaviConnected {
        ticket: bot.id.clone(),
        name: bot.name.clone(),
        texture_path: bot.texture_path.clone(),
        animation_path: bot.animation_path.clone(),
        x: bot.x,
        y: bot.y,
        z: bot.z,
        warp_in: false,
      });
    }

    // todo: send position
    packets.push(ServerPacket::Login {
      ticket: player_id.clone(),
    });

    let player = self.players.get_mut(player_id).unwrap();
    let mut asset_packets = Vec::new();

    for asset_path in asset_paths {
      if let Some(asset) = self.assets.get(&asset_path) {
        asset_packets.extend(create_asset_stream(&asset_path, asset));
      } else {
        println!("Missing asset! \"{}\"", asset_path);
      }

      player.cached_assets.insert(asset_path);
    }

    // send asset_packets before anything else
    asset_packets.append(&mut packets);
    let packets = asset_packets;

    for packet in packets {
      player
        .packet_shipper
        .send(&self.socket, &Reliability::ReliableOrdered, &packet)?;
    }

    Ok(())
  }

  pub(super) fn mark_player_ready(&mut self, id: &String) {
    if let Some(player) = self.players.get_mut(id) {
      player.ready = true;

      // clone id to end mutable player lifetime
      let player_id = player.navi.id.clone();
      let area = self.areas.get_mut(&player.navi.area_id).unwrap();
      let texture_path = player.navi.texture_path.clone();
      let animation_path = player.navi.animation_path.clone();

      let packet = ServerPacket::NaviConnected {
        ticket: player_id.clone(),
        name: player.navi.name.clone(),
        texture_path: texture_path.clone(),
        animation_path: animation_path.clone(),
        x: player.navi.x,
        y: player.navi.y,
        z: player.navi.z,
        warp_in: true,
      };

      assert_asset(
        &self.socket,
        area,
        &self.assets,
        &mut self.players,
        &texture_path,
      );

      assert_asset(
        &self.socket,
        area,
        &self.assets,
        &mut self.players,
        &animation_path,
      );

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
    use super::asset;

    self.assets.remove(&asset::get_player_texture_path(id));
    self.assets.remove(&asset::get_player_animation_path(id));

    if let Some(player) = self.players.remove(id) {
      let area = self
        .areas
        .get_mut(&player.navi.area_id)
        .expect("Missing area for removed player");

      area.remove_player(&player.navi.id);

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

  pub fn add_bot(&mut self, bot: Navi) {
    if let Some(area) = self.areas.get_mut(&bot.area_id) {
      area.add_bot(bot.id.clone());

      let packet = ServerPacket::NaviConnected {
        ticket: bot.id.clone(),
        name: bot.name.clone(),
        texture_path: bot.texture_path.clone(),
        animation_path: bot.animation_path.clone(),
        x: bot.x,
        y: bot.y,
        z: bot.z,
        warp_in: true,
      };

      assert_asset(
        &self.socket,
        area,
        &self.assets,
        &mut self.players,
        &bot.texture_path,
      );

      assert_asset(
        &self.socket,
        area,
        &self.assets,
        &mut self.players,
        &bot.animation_path,
      );

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

  pub fn get_bot(&self, id: &String) -> Option<&Navi> {
    self.bots.get(id)
  }

  pub fn set_bot_name(&mut self, id: &String, name: String) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.name = name.clone();

      let packet = ServerPacket::NaviSetName {
        ticket: id.clone(),
        name,
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

  pub fn set_bot_avatar(&mut self, id: &String, texture_path: String, animation_path: String) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.texture_path = texture_path.clone();
      bot.animation_path = animation_path.clone();

      let area = self.areas.get(&bot.area_id).unwrap();

      assert_asset(
        &self.socket,
        area,
        &self.assets,
        &mut self.players,
        &texture_path,
      );

      assert_asset(
        &self.socket,
        area,
        &self.assets,
        &mut self.players,
        &animation_path,
      );

      let packet = ServerPacket::NaviSetAvatar {
        ticket: id.clone(),
        texture_path: texture_path,
        animation_path: animation_path,
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
    use super::asset::get_map_path;

    for area in self.areas.values_mut() {
      let map = area.get_map_mut();

      if map.is_dirty() {
        let map_path = get_map_path(map.get_name());
        let map_asset = Asset::Text(map.render());

        update_cached_players(&self.socket, &mut self.players, &map_path, &map_asset);
        self.assets.insert(map_path.clone(), map_asset);

        let packet = ServerPacket::MapUpdate { map_path };

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

fn update_cached_players(
  socket: &UdpSocket,
  players: &mut HashMap<String, Player>,
  path: &String,
  asset: &Asset,
) {
  let reliability = Reliability::ReliableOrdered;
  let packets = create_asset_stream(path, &asset);

  for player in players.values_mut() {
    if player.cached_assets.contains(path) {
      for packet in &packets {
        // todo: handle in packet_shipper?
        let _ = player.packet_shipper.send(socket, &reliability, &packet);
      }
    }
  }
}

fn assert_asset(
  socket: &UdpSocket,
  area: &Area,
  assets: &HashMap<String, Asset>,
  players: &mut HashMap<String, Player>,
  asset_path: &String,
) {
  if let Some(asset) = assets.get(asset_path) {
    let player_ids = area.get_connected_players();

    let packets = create_asset_stream(asset_path, asset);

    for player_id in player_ids {
      let player = players.get_mut(player_id).unwrap();

      if player.cached_assets.contains(asset_path) {
        continue;
      }

      for packet in &packets {
        let _ = player
          .packet_shipper
          .send(socket, &Reliability::ReliableOrdered, &packet);
      }
    }
  } else {
    println!("Missing asset! \"{}\"", asset_path);
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
