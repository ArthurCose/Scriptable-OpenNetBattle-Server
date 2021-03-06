use super::client::Client;
use super::map::Map;
use super::server::ServerConfig;
use super::{Area, Asset, AssetData, Navi};
use crate::packets::{create_asset_stream, PacketShipper, Reliability, ServerPacket};
use std::collections::{HashMap, HashSet};
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Net {
  socket: Rc<UdpSocket>,
  max_payload_size: usize,
  resend_budget: usize,
  areas: HashMap<String, Area>,
  clients: HashMap<String, Client>,
  bots: HashMap<String, Navi>,
  assets: HashMap<String, Asset>,
}

impl Net {
  pub fn new(socket: Rc<UdpSocket>, server_config: &ServerConfig) -> Net {
    use super::asset::get_map_path;
    use std::fs::{read_dir, read_to_string};

    let mut assets = HashMap::new();
    Net::load_assets_from_dir(&mut assets, &std::path::Path::new("assets"));

    let mut areas = HashMap::new();
    let mut default_area_provided = false;

    for wrapped_dir_entry in read_dir("./areas").expect("Area folder missing! (./areas)") {
      if let Ok(map_dir_entry) = wrapped_dir_entry {
        let map_path = map_dir_entry.path();
        let area_id = map_path
          .file_stem()
          .unwrap_or_default()
          .to_string_lossy()
          .into_owned();

        if let Ok(raw_map) = read_to_string(&map_path) {
          let mut map = Map::from(raw_map);

          if area_id == "default" {
            default_area_provided = true
          }

          let map_asset = map.generate_asset();

          assets.insert(get_map_path(&area_id), map_asset);
          areas.insert(area_id.clone(), Area::new(area_id, map));
        }
      }
    }

    if !default_area_provided {
      panic!("No default (default.txt) area data found");
    }

    Net {
      socket,
      max_payload_size: server_config.max_payload_size,
      resend_budget: server_config.resend_budget,
      areas,
      clients: HashMap::new(),
      bots: HashMap::new(),
      assets,
    }
  }

  fn load_assets_from_dir(assets: &mut HashMap<String, Asset>, dir: &std::path::Path) {
    use super::{resolve_tsx_dependencies, translate_tsx};
    use std::fs::{read, read_dir, read_to_string};

    if let Ok(entries) = read_dir(dir) {
      for wrapped_entry in entries {
        if let Ok(entry) = wrapped_entry {
          let path = entry.path();

          if path.is_dir() {
            Net::load_assets_from_dir(assets, &path);
          } else {
            let path_string = String::from("/server/") + path.to_str().unwrap_or_default();
            let extension_index = path_string.rfind('.').unwrap_or_else(|| path_string.len());
            let extension = path_string.to_lowercase().split_off(extension_index);

            let asset_data = if extension == ".ogg" {
              AssetData::Audio(read(&path).unwrap_or_default())
            } else if extension == ".png" || extension == ".bmp" {
              AssetData::Texture(read(&path).unwrap_or_default())
            } else if extension == ".tsx" {
              let original_data = read_to_string(&path).unwrap_or_default();
              let translated_data = translate_tsx(&path, &original_data);

              if translated_data == None {
                println!("Invalid .tsx file: {:?}", path);
              }

              AssetData::Text(translated_data.unwrap_or(original_data))
            } else {
              AssetData::Text(read_to_string(&path).unwrap_or_default())
            };

            let mut dependencies = Vec::new();

            if extension == ".tsx" {
              // can't chain yet: https://github.com/rust-lang/rust/issues/53667
              if let AssetData::Text(data) = &asset_data {
                dependencies = resolve_tsx_dependencies(data);
              }
            }

            let asset = Asset {
              data: asset_data,
              dependencies,
            };

            assets.insert(path_string, asset);
          }
        }
      }
    }
  }

  #[allow(dead_code)]
  pub fn get_area(&self, area_id: &str) -> Option<&Area> {
    self.areas.get(area_id)
  }

  pub fn get_area_mut(&mut self, area_id: &str) -> Option<&mut Area> {
    self.areas.get_mut(area_id)
  }

  #[allow(dead_code)]
  pub fn get_asset(&self, path: &str) -> Option<&Asset> {
    self.assets.get(path)
  }

  pub fn set_asset(&mut self, path: String, asset: Asset) {
    self.assets.insert(path.clone(), asset);

    update_cached_clients(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &path,
    );
  }

  #[allow(dead_code)]
  pub fn remove_asset(&mut self, path: &str) {
    self.assets.remove(path);
  }

  pub fn get_player(&self, id: &str) -> Option<&Navi> {
    self.clients.get(id).map(|client| &client.navi)
  }

  pub(super) fn get_client(&self, id: &str) -> Option<&Client> {
    self.clients.get(id)
  }

  pub(super) fn get_client_mut(&mut self, id: &str) -> Option<&mut Client> {
    self.clients.get_mut(id)
  }

  pub fn set_player_name(&mut self, id: &str, name: String) {
    if let Some(client) = self.clients.get_mut(id) {
      client.navi.name = name.clone();

      // skip if client has not even been sent to anyone yet
      if client.ready {
        let packet = ServerPacket::NaviSetName {
          ticket: id.to_string(),
          name,
        };

        let area = self.areas.get(&client.navi.area_id).unwrap();

        broadcast_to_area(
          &self.socket,
          &mut self.clients,
          area,
          Reliability::Reliable,
          packet,
        );
      }
    }
  }

  pub fn lock_player_input(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::LockInput,
      );
    }
  }

  pub fn unlock_player_input(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::UnlockInput,
      );
    }
  }

  pub fn move_player(&mut self, id: &str, x: f32, y: f32, z: f32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::Move { x, y, z },
      );

      // don't update internal position, allow the client to update this
    }
  }

  pub(crate) fn update_player_position(&mut self, id: &str, x: f32, y: f32, z: f32) {
    let client = self.clients.get_mut(id).unwrap();
    client.navi.x = x;
    client.navi.y = y;
    client.navi.z = z;

    // skip if client has not even been sent to anyone yet
    if !client.ready {
      return;
    }

    let packet = ServerPacket::NaviMove {
      ticket: id.to_string(),
      x,
      y,
      z,
    };

    let area = self.areas.get(&client.navi.area_id).unwrap();

    broadcast_to_area(
      &self.socket,
      &mut self.clients,
      area,
      Reliability::UnreliableSequenced,
      packet,
    );
  }

  pub fn message_player(
    &mut self,
    id: &str,
    message: &str,
    mug_texture_path: &str,
    mug_animation_path: &str,
  ) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::Message {
          message: message.to_string(),
          mug_texture_path: mug_texture_path.to_string(),
          mug_animation_path: mug_animation_path.to_string(),
        },
      );
    }
  }

  pub fn question_player(
    &mut self,
    id: &str,
    message: &str,
    mug_texture_path: &str,
    mug_animation_path: &str,
  ) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::Question {
          message: message.to_string(),
          mug_texture_path: mug_texture_path.to_string(),
          mug_animation_path: mug_animation_path.to_string(),
        },
      );
    }
  }

  pub fn quiz_player(
    &mut self,
    id: &str,
    option_a: &str,
    option_b: &str,
    option_c: &str,
    mug_texture_path: &str,
    mug_animation_path: &str,
  ) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::Quiz {
          option_a: option_a.to_string(),
          option_b: option_b.to_string(),
          option_c: option_c.to_string(),
          mug_texture_path: mug_texture_path.to_string(),
          mug_animation_path: mug_animation_path.to_string(),
        },
      );
    }
  }

  pub fn transfer_player(
    &mut self,
    id: &str,
    area_id: &str,
    warp_in: bool,
    x: f32,
    y: f32,
    z: f32,
  ) {
    if self.areas.get(area_id).is_none() {
      // non existent area
      return;
    }

    let texture_path;
    let animation_path;

    if let Some(client) = self.clients.get_mut(id) {
      let previous_area = self.areas.get_mut(&client.navi.area_id).unwrap();

      if !previous_area
        .get_connected_players()
        .contains(&id.to_string())
      {
        // client has not been added to any area yet
        // assume client was transferred on initial connection by a plugin
        client.navi.area_id = area_id.to_string();
        return;
      }

      texture_path = client.navi.texture_path.clone();
      animation_path = client.navi.animation_path.clone();

      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::TransferStart,
      );

      let previous_area = self.areas.get_mut(&client.navi.area_id).unwrap();
      previous_area.remove_player(&id);

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        previous_area,
        Reliability::Reliable,
        ServerPacket::NaviDisconnected {
          ticket: id.to_string(),
          warp_out: warp_in,
        },
      );
    } else {
      // allows us to safely unwrap after this
      // as long as send_area doesn't delete the client (why would it?)
      return;
    }

    let area = self.areas.get_mut(area_id).unwrap();
    area.add_player(id.to_string());

    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      area.get_connected_players(),
      &texture_path,
    );

    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      area.get_connected_players(),
      &animation_path,
    );

    self.send_area(id, &area_id);

    let mut client = self.clients.get_mut(id).unwrap();

    client.navi.area_id = area_id.to_string();
    client.warp_in = warp_in;

    client.packet_shipper.send(
      &self.socket,
      &Reliability::ReliableOrdered,
      &ServerPacket::Move { x, y, z },
    );

    client.packet_shipper.send(
      &self.socket,
      &Reliability::ReliableOrdered,
      &ServerPacket::TransferComplete,
    );
  }

  pub fn set_player_avatar(&mut self, id: &str, texture_path: String, animation_path: String) {
    if let Some(client) = self.clients.get_mut(id) {
      client.navi.texture_path = texture_path.clone();
      client.navi.animation_path = animation_path.clone();

      let area = self.areas.get(&client.navi.area_id).unwrap();

      // skip if client has not even been sent to anyone yet
      if client.ready {
        update_cached_clients(
          &self.socket,
          self.max_payload_size,
          &self.assets,
          &mut self.clients,
          &texture_path,
        );

        update_cached_clients(
          &self.socket,
          self.max_payload_size,
          &self.assets,
          &mut self.clients,
          &animation_path,
        );

        let packet = ServerPacket::NaviSetAvatar {
          ticket: id.to_string(),
          texture_path,
          animation_path,
        };

        broadcast_to_area(
          &self.socket,
          &mut self.clients,
          area,
          Reliability::ReliableOrdered,
          packet,
        );
      }
    }
  }

  pub fn player_emote(&mut self, id: &str, emote_id: u8) {
    if let Some(client) = self.clients.get(id) {
      let packet = ServerPacket::NaviEmote {
        ticket: id.to_string(),
        emote_id,
      };

      let area = self.areas.get(&client.navi.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
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
  ) -> String {
    use uuid::Uuid;

    let id = Uuid::new_v4().to_string();

    let (texture_path, animation_path) =
      self.store_player_avatar(&id, texture_data, animation_data);

    let area_id = String::from("default");
    let area = self.get_area_mut(&area_id).unwrap();
    let (spawn_x, spawn_y) = area.get_map().get_spawn();

    let client = Client {
      socket_address,
      packet_shipper: PacketShipper::new(socket_address, self.resend_budget),
      navi: Navi {
        id: id.clone(),
        name,
        area_id,
        texture_path,
        animation_path,
        x: spawn_x,
        y: spawn_y,
        z: 0.0,
        solid: false,
      },
      warp_in: true,
      ready: false,
      cached_assets: HashSet::new(),
    };

    self.clients.insert(client.navi.id.clone(), client);

    id
  }

  pub(super) fn store_player_avatar(
    &mut self,
    player_id: &str,
    texture_data: Vec<u8>,
    animation_data: String,
  ) -> (String, String) {
    use super::asset;

    let texture_path = asset::get_player_texture_path(player_id);
    let animation_path = asset::get_player_animation_path(player_id);

    self.set_asset(
      texture_path.clone(),
      Asset {
        data: AssetData::SFMLImage(texture_data),
        dependencies: Vec::new(),
      },
    );

    self.set_asset(
      animation_path.clone(),
      Asset {
        data: AssetData::Text(animation_data),
        dependencies: Vec::new(),
      },
    );

    (texture_path, animation_path)
  }

  pub(super) fn connect_client(&mut self, player_id: &str) {
    let client = self.clients.get(player_id).unwrap();
    let area_id = client.navi.area_id.clone();
    let texture_path = client.navi.texture_path.clone();
    let animation_path = client.navi.animation_path.clone();

    let area = self.areas.get_mut(&area_id).unwrap();
    area.add_player(client.navi.id.clone());

    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      area.get_connected_players(),
      &texture_path,
    );

    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      area.get_connected_players(),
      &animation_path,
    );

    self.send_area(player_id, &area_id);

    // todo: send position
    let packet = ServerPacket::Login {
      ticket: player_id.to_string(),
    };

    let client = self.clients.get_mut(player_id).unwrap();

    client
      .packet_shipper
      .send(&self.socket, &Reliability::ReliableOrdered, &packet);
  }

  fn send_area(&mut self, player_id: &str, area_id: &str) {
    use super::asset::get_map_path;

    let mut packets: Vec<ServerPacket> = Vec::new();
    let mut asset_paths: Vec<String> = Vec::new();

    // send map
    let map_path = get_map_path(area_id);
    asset_paths.push(map_path.clone());
    packets.push(ServerPacket::MapUpdate { map_path });

    let area = self.areas.get(area_id).unwrap();

    // send clients
    for other_player_id in area.get_connected_players() {
      if other_player_id == player_id {
        continue;
      }

      let other_client = self.clients.get(other_player_id).unwrap();

      asset_paths.push(other_client.navi.texture_path.clone());
      asset_paths.push(other_client.navi.animation_path.clone());

      packets.push(ServerPacket::NaviConnected {
        ticket: other_client.navi.id.clone(),
        name: other_client.navi.name.clone(),
        texture_path: other_client.navi.texture_path.clone(),
        animation_path: other_client.navi.animation_path.clone(),
        x: other_client.navi.x,
        y: other_client.navi.y,
        z: other_client.navi.z,
        solid: other_client.navi.solid,
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
        solid: bot.solid,
        warp_in: false,
      });
    }

    // send asset_packets before anything else
    let asset_recievers = vec![player_id.to_string()];

    for asset_path in asset_paths {
      assert_asset(
        &self.socket,
        self.max_payload_size,
        &self.assets,
        &mut self.clients,
        &&asset_recievers[..],
        &asset_path,
      );
    }

    let client = self.clients.get_mut(player_id).unwrap();

    for packet in packets {
      client
        .packet_shipper
        .send(&self.socket, &Reliability::ReliableOrdered, &packet);
    }
  }

  // handles first join and completed transfer
  pub(super) fn mark_client_ready(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.ready = true;

      // clone id to end mutable client lifetime
      let player_id = client.navi.id.clone();
      let area = self.areas.get_mut(&client.navi.area_id).unwrap();
      let texture_path = client.navi.texture_path.clone();
      let animation_path = client.navi.animation_path.clone();

      let packet = ServerPacket::NaviConnected {
        ticket: player_id,
        name: client.navi.name.clone(),
        texture_path,
        animation_path,
        x: client.navi.x,
        y: client.navi.y,
        z: client.navi.z,
        solid: client.navi.solid,
        warp_in: client.warp_in,
      };

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::ReliableOrdered,
        packet,
      );
    }
  }

  pub fn remove_player(&mut self, id: &str) {
    use super::asset;

    self.assets.remove(&asset::get_player_texture_path(id));
    self.assets.remove(&asset::get_player_animation_path(id));

    if let Some(client) = self.clients.remove(id) {
      let area = self
        .areas
        .get_mut(&client.navi.area_id)
        .expect("Missing area for removed client");

      area.remove_player(&client.navi.id);

      let packet = ServerPacket::NaviDisconnected {
        ticket: id.to_string(),
        warp_out: true,
      };

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
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
        solid: bot.solid,
        warp_in: true,
      };

      assert_asset(
        &self.socket,
        self.max_payload_size,
        &self.assets,
        &mut self.clients,
        area.get_connected_players(),
        &bot.texture_path,
      );

      assert_asset(
        &self.socket,
        self.max_payload_size,
        &self.assets,
        &mut self.clients,
        area.get_connected_players(),
        &bot.animation_path,
      );

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::ReliableOrdered,
        packet,
      );

      self.bots.insert(bot.id.clone(), bot);
    }
  }

  pub fn get_bot(&self, id: &str) -> Option<&Navi> {
    self.bots.get(id)
  }

  pub fn set_bot_name(&mut self, id: &str, name: String) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.name = name.clone();

      let packet = ServerPacket::NaviSetName {
        ticket: id.to_string(),
        name,
      };

      let area = self.areas.get(&bot.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub fn move_bot(&mut self, id: &str, x: f32, y: f32, z: f32) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.x = x;
      bot.y = y;
      bot.z = z;
    }
  }

  pub fn transfer_bot(&mut self, id: &str, area_id: &str, warp_in: bool, x: f32, y: f32, z: f32) {
    if self.areas.get(area_id).is_none() {
      // non existent area
      return;
    }

    if let Some(bot) = self.bots.get_mut(id) {
      let previous_area = self.areas.get_mut(&bot.area_id).unwrap();
      previous_area.remove_bot(&id);

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        previous_area,
        Reliability::Reliable,
        ServerPacket::NaviDisconnected {
          ticket: id.to_string(),
          warp_out: warp_in,
        },
      );

      bot.area_id = area_id.to_string();
      bot.x = x;
      bot.y = y;
      bot.z = z;

      let area = self.areas.get_mut(area_id).unwrap();
      area.add_bot(id.to_string());

      assert_asset(
        &self.socket,
        self.max_payload_size,
        &self.assets,
        &mut self.clients,
        area.get_connected_players(),
        &bot.texture_path,
      );

      assert_asset(
        &self.socket,
        self.max_payload_size,
        &self.assets,
        &mut self.clients,
        area.get_connected_players(),
        &bot.animation_path,
      );

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        ServerPacket::NaviConnected {
          ticket: id.to_string(),
          name: bot.name.clone(),
          texture_path: bot.texture_path.clone(),
          animation_path: bot.animation_path.clone(),
          x: bot.x,
          y: bot.y,
          z: bot.z,
          solid: bot.solid,
          warp_in,
        },
      );
    }
  }

  pub fn set_bot_avatar(&mut self, id: &str, texture_path: String, animation_path: String) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.texture_path = texture_path.clone();
      bot.animation_path = animation_path.clone();

      let area = self.areas.get(&bot.area_id).unwrap();

      update_cached_clients(
        &self.socket,
        self.max_payload_size,
        &self.assets,
        &mut self.clients,
        &texture_path,
      );

      update_cached_clients(
        &self.socket,
        self.max_payload_size,
        &self.assets,
        &mut self.clients,
        &animation_path,
      );

      let packet = ServerPacket::NaviSetAvatar {
        ticket: id.to_string(),
        texture_path,
        animation_path,
      };

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::ReliableOrdered,
        packet,
      );
    }
  }

  pub fn set_bot_emote(&mut self, id: &str, emote_id: u8) {
    if let Some(bot) = self.bots.get(id) {
      let packet = ServerPacket::NaviEmote {
        ticket: id.to_string(),
        emote_id,
      };

      let area = self.areas.get(&bot.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub fn remove_bot(&mut self, id: &str) {
    if let Some(bot) = self.bots.remove(id) {
      let area = self
        .areas
        .get_mut(&bot.area_id)
        .expect("Missing area for removed bot");

      area.remove_bot(&bot.id);

      let packet = ServerPacket::NaviDisconnected {
        ticket: id.to_string(),
        warp_out: true,
      };

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub(super) fn tick(&mut self) {
    self.resend_backed_up_packets();
    self.broadcast_bot_positions();
    self.broadcast_map_changes();
  }

  fn broadcast_bot_positions(&mut self) {
    for bot in self.bots.values() {
      let packet = ServerPacket::NaviMove {
        ticket: bot.id.clone(),
        x: bot.x,
        y: bot.y,
        z: bot.z,
      };

      let area = self.areas.get(&bot.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::UnreliableSequenced,
        packet,
      );
    }
  }

  fn broadcast_map_changes(&mut self) {
    use super::asset::get_map_path;

    for area in self.areas.values_mut() {
      let map_path = get_map_path(area.get_id());
      let map = area.get_map_mut();

      if map.is_dirty() {
        let map_asset = map.generate_asset();

        self.assets.insert(map_path.clone(), map_asset);
        update_cached_clients(
          &self.socket,
          self.max_payload_size,
          &self.assets,
          &mut self.clients,
          &map_path,
        );

        let packet = ServerPacket::MapUpdate { map_path };

        broadcast_to_area(
          &self.socket,
          &mut self.clients,
          area,
          Reliability::ReliableOrdered,
          packet,
        );
      }
    }
  }

  fn resend_backed_up_packets(&mut self) {
    for client in self.clients.values_mut() {
      client.packet_shipper.resend_backed_up_packets(&self.socket);
    }
  }
}

fn update_cached_clients(
  socket: &UdpSocket,
  max_payload_size: usize,
  assets: &HashMap<String, Asset>,
  clients: &mut HashMap<String, Client>,
  asset_path: &str,
) {
  use super::get_flattened_dependency_chain;
  let mut dependencies = get_flattened_dependency_chain(assets, asset_path);
  dependencies.pop();

  let reliability = Reliability::ReliableOrdered;

  let mut clients_to_update: Vec<&mut Client> = clients
    .values_mut()
    .filter(|client| client.cached_assets.contains(asset_path))
    .collect();

  // asserting dependencies
  for asset_path in dependencies {
    if let Some(asset) = assets.get(asset_path) {
      let mut packets = Vec::new();

      for client in &mut clients_to_update {
        if client.cached_assets.contains(asset_path) {
          continue;
        }

        client.cached_assets.insert(asset_path.to_string());

        // lazily create stream
        if packets.is_empty() {
          packets = create_asset_stream(max_payload_size, asset_path, &asset);
        }

        for packet in &packets {
          client.packet_shipper.send(socket, &reliability, &packet);
        }
      }
    }
  }

  // updating clients who have this asset
  if let Some(asset) = assets.get(asset_path) {
    let packets = create_asset_stream(max_payload_size, asset_path, &asset);

    for client in &mut clients_to_update {
      for packet in &packets {
        client.packet_shipper.send(socket, &reliability, &packet);
      }
    }
  }
}

fn assert_asset(
  socket: &UdpSocket,
  max_payload_size: usize,
  assets: &HashMap<String, Asset>,
  clients: &mut HashMap<String, Client>,
  player_ids: &[String],
  asset_path: &str,
) {
  use super::get_flattened_dependency_chain;
  let assets_to_send = get_flattened_dependency_chain(assets, asset_path);

  for asset_path in assets_to_send {
    let asset = assets.get(asset_path).unwrap();

    let mut packets: Vec<ServerPacket> = Vec::new();

    for player_id in player_ids {
      let client = clients.get_mut(player_id).unwrap();

      if client.cached_assets.contains(asset_path) {
        continue;
      }

      // lazily create stream
      if packets.is_empty() {
        packets = create_asset_stream(max_payload_size, asset_path, asset);
      }

      client.cached_assets.insert(asset_path.to_string());

      for packet in &packets {
        client
          .packet_shipper
          .send(socket, &Reliability::ReliableOrdered, &packet);
      }
    }
  }
}

fn broadcast_to_area(
  socket: &UdpSocket,
  clients: &mut HashMap<String, Client>,
  area: &Area,
  reliability: Reliability,
  packet: ServerPacket,
) {
  for player_id in area.get_connected_players() {
    let client = clients.get_mut(player_id).unwrap();

    client.packet_shipper.send(socket, &reliability, &packet);
  }
}
