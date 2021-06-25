use super::actor_property_animation::KeyFrame;
use super::boot::Boot;
use super::client::Client;
use super::map::Map;
use super::server::ServerConfig;
use super::{Actor, Area, Asset, AssetData, BbsPost, Direction, PlayerData};
use crate::packets::{create_asset_stream, Reliability, ServerPacket};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Net {
  socket: Rc<UdpSocket>,
  config: Rc<ServerConfig>,
  areas: HashMap<String, Area>,
  clients: HashMap<String, Client>,
  bots: HashMap<String, Actor>,
  assets: HashMap<String, Asset>,
  active_script: usize,
  kick_list: Vec<Boot>,
}

impl Net {
  pub fn new(socket: Rc<UdpSocket>, config: Rc<ServerConfig>) -> Net {
    use super::asset::get_map_path;
    use std::fs::{read_dir, read_to_string};

    let mut assets = HashMap::new();
    Net::load_assets_from_dir(&mut assets, &std::path::Path::new("assets"));

    let mut areas = HashMap::new();
    let mut default_area_provided = false;

    for map_dir_entry in read_dir("./areas")
      .expect("Area folder missing! (./areas)")
      .flatten()
    {
      let map_path = map_dir_entry.path();
      let area_id = map_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

      if let Ok(raw_map) = read_to_string(&map_path) {
        let mut map = Map::from(&raw_map);

        if area_id == "default" {
          default_area_provided = true
        }

        let map_asset = map.generate_asset();

        assets.insert(get_map_path(&area_id), map_asset);
        areas.insert(area_id.clone(), Area::new(area_id, map));
      }
    }

    if !default_area_provided {
      panic!("No default (default.txt) area data found");
    }

    Net {
      socket,
      config,
      areas,
      clients: HashMap::new(),
      bots: HashMap::new(),
      assets,
      active_script: 0,
      kick_list: Vec::new(),
    }
  }

  fn load_assets_from_dir(assets: &mut HashMap<String, Asset>, dir: &std::path::Path) {
    use super::load_asset;
    use std::fs::read_dir;

    if let Ok(entries) = read_dir(dir) {
      for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
          Net::load_assets_from_dir(assets, &path);
        } else {
          let mut path_string = String::from("/server/") + path.to_str().unwrap_or_default();

          // adjust windows paths
          path_string = path_string.replace('\\', "/");

          assets.insert(path_string, load_asset(path));
        }
      }
    }
  }

  pub fn get_asset(&self, path: &str) -> Option<&Asset> {
    self.assets.get(path)
  }

  pub fn set_asset(&mut self, path: String, asset: Asset) {
    self.assets.insert(path.clone(), asset);

    update_cached_clients(
      &self.socket,
      self.config.max_payload_size,
      &self.assets,
      &mut self.clients,
      &path,
    );
  }

  pub fn get_areas(&self) -> impl std::iter::Iterator<Item = &Area> {
    self.areas.values()
  }

  pub fn get_area(&self, area_id: &str) -> Option<&Area> {
    self.areas.get(area_id)
  }

  pub fn get_area_mut(&mut self, area_id: &str) -> Option<&mut Area> {
    self.areas.get_mut(area_id)
  }

  pub fn add_area(&mut self, id: String, map: Map) {
    use super::asset::get_map_path;

    let mut map = map;

    if let Some(area) = self.areas.get_mut(&id) {
      area.set_map(map);
    } else {
      self.assets.insert(get_map_path(&id), map.generate_asset());
      self.areas.insert(id.clone(), Area::new(id, map));
    }
  }

  pub fn remove_area(&mut self, id: &str) {
    use super::asset::get_map_path;

    let area_optional = self.areas.remove(id);
    self.assets.remove(&get_map_path(id));

    if let Some(area) = area_optional {
      let player_ids = area.get_connected_players();

      for player_id in player_ids {
        self.kick_player(player_id, "Area destroyed", true);
      }
    }
  }

  pub fn remove_asset(&mut self, path: &str) {
    self.assets.remove(path);
  }

  pub fn get_player(&self, id: &str) -> Option<&Actor> {
    self.clients.get(id).map(|client| &client.actor)
  }

  pub fn get_player_addr(&self, id: &str) -> Option<std::net::SocketAddr> {
    self.clients.get(id).map(|client| client.socket_address)
  }

  #[allow(dead_code)]
  pub(super) fn get_client(&self, id: &str) -> Option<&Client> {
    self.clients.get(id)
  }

  pub(super) fn get_client_mut(&mut self, id: &str) -> Option<&mut Client> {
    self.clients.get_mut(id)
  }

  pub fn require_asset(&mut self, area_id: &str, asset_path: &str) {
    if let Some(area) = self.areas.get_mut(area_id) {
      assert_asset(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        &area.get_connected_players(),
        asset_path,
      );

      area.require_asset(asset_path.to_string());
    }
  }

  pub fn play_sound(&mut self, area_id: &str, path: &str) {
    if let Some(area) = self.areas.get(area_id) {
      assert_asset(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        &area.get_connected_players(),
        path,
      );

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        ServerPacket::PlaySound {
          path: path.to_string(),
        },
      )
    }
  }

  pub fn set_player_name(&mut self, id: &str, name: String) {
    if let Some(client) = self.clients.get_mut(id) {
      client.actor.name = name.clone();

      // skip if client has not even been sent to anyone yet
      if client.ready {
        let packet = ServerPacket::ActorSetName {
          ticket: id.to_string(),
          name,
        };

        let area = self.areas.get(&client.actor.area_id).unwrap();

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

  pub fn set_player_avatar(&mut self, id: &str, texture_path: String, animation_path: String) {
    if let Some(client) = self.clients.get_mut(id) {
      client.actor.texture_path = texture_path.clone();
      client.actor.animation_path = animation_path.clone();

      let area = self.areas.get(&client.actor.area_id).unwrap();

      // we'd normally skip if the player has not been sent to anyone yet
      // but for this we want to make sure the player sees this and updates their avatar
      // if the other players receive this, they'll just ignore it

      assert_assets(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        area.get_connected_players(),
        [texture_path.as_str(), animation_path.as_str()].iter(),
      );

      let packet = ServerPacket::ActorSetAvatar {
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

  pub fn set_player_emote(&mut self, id: &str, emote_id: u8, use_custom_emotes: bool) {
    if let Some(client) = self.clients.get(id) {
      let packet = ServerPacket::ActorEmote {
        ticket: id.to_string(),
        emote_id,
        use_custom_emotes,
      };

      let area = self.areas.get(&client.actor.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        packet,
      );
    }
  }

  pub fn exclusive_player_emote(
    &mut self,
    target_id: &str,
    emoter_id: &str,
    emote_id: u8,
    use_custom_emotes: bool,
  ) {
    if let Some(client) = self.clients.get_mut(target_id) {
      let packet = ServerPacket::ActorEmote {
        ticket: emoter_id.to_string(),
        emote_id,
        use_custom_emotes,
      };

      client
        .packet_shipper
        .send(&self.socket, Reliability::Reliable, &packet);
    }
  }

  pub fn animate_player(&mut self, id: &str, name: &str, loop_animation: bool) {
    if let Some(client) = self.clients.get(id) {
      let area = self.areas.get(&client.actor.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        ServerPacket::ActorAnimate {
          ticket: id.to_string(),
          state: name.to_string(),
          loop_animation,
        },
      );
    }
  }

  pub fn animate_player_properties(&mut self, id: &str, animation: Vec<KeyFrame>) {
    use super::actor_property_animation::ActorProperty;
    use std::collections::HashSet;

    if let Some(client) = self.clients.get_mut(id) {
      let area = match self.areas.get(&client.actor.area_id) {
        Some(area) => area,
        None => return,
      };

      let mut asset_paths = HashSet::<&str>::new();

      // store final values for new players, also track assets
      for keyframe in &animation {
        for (property, _) in &keyframe.property_steps {
          match property {
            ActorProperty::Animation(value) => client.actor.current_animation = Some(value.clone()),
            ActorProperty::ScaleX(value) => client.actor.scale_x = *value,
            ActorProperty::ScaleY(value) => client.actor.scale_y = *value,
            ActorProperty::Rotation(value) => client.actor.rotation = *value,
            ActorProperty::Direction(value) => client.actor.direction = *value,
            ActorProperty::SoundEffect(value) | ActorProperty::SoundEffectLoop(value) => {
              if value.starts_with("/server/") {
                asset_paths.insert(value);
              }
            }
            _ => {}
          }
        }
      }

      assert_assets(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        &[id.to_string()],
        asset_paths.iter(),
      );

      broadcast_actor_keyframes(
        &self.socket,
        &mut self.clients,
        area,
        self.config.max_payload_size,
        id,
        animation,
      );
    }
  }

  pub fn is_player_in_widget(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_in_widget();
    }

    false
  }

  pub fn preload_asset_for_player(&mut self, id: &str, asset_path: &str) {
    assert_asset(
      &self.socket,
      self.config.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[String::from(id)],
      asset_path,
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::Preload {
          asset_path: asset_path.to_string(),
        },
      );
    }
  }

  pub fn play_sound_for_player(&mut self, id: &str, path: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::PlaySound {
          path: path.to_string(),
        },
      );
    }
  }

  pub fn exclude_object_for_player(&mut self, id: &str, object_id: u32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::ExcludeObject { id: object_id },
      );
    }
  }

  pub fn include_object_for_player(&mut self, id: &str, object_id: u32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::IncludeObject { id: object_id },
      );
    }
  }

  pub fn move_player_camera(&mut self, id: &str, x: f32, y: f32, z: f32, hold_time: f32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::MoveCamera { x, y, z, hold_time },
      );
    }
  }

  pub fn slide_player_camera(&mut self, id: &str, x: f32, y: f32, z: f32, duration: f32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::SlideCamera { x, y, z, duration },
      );
    }
  }

  pub fn shake_player_camera(&mut self, id: &str, strength: f32, duration: f32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::ShakeCamera { strength, duration },
      );
    }
  }

  pub fn track_with_player_camera(&mut self, id: &str, actor_id: Option<String>) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::TrackWithCamera { actor_id },
      );
    }
  }

  pub fn unlock_player_camera(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::UnlockCamera,
      );
    }
  }

  pub fn lock_player_input(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::LockInput,
      );
    }
  }

  pub fn unlock_player_input(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::UnlockInput,
      );
    }
  }

  pub fn teleport_player(
    &mut self,
    id: &str,
    warp: bool,
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  ) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::Teleport {
          warp,
          x,
          y,
          z,
          direction,
        },
      );

      // set their warp position
      // useful for moving players requesting/connecting
      client.warp_x = x;
      client.warp_y = y;
      client.warp_z = z;
      client.warp_direction = direction;

      // don't update internal position, allow the client to update this
    }
  }

  pub(crate) fn update_player_position(
    &mut self,
    id: &str,
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  ) {
    let client = self.clients.get_mut(id).unwrap();
    client.actor.x = x;
    client.actor.y = y;
    client.actor.z = z;
    client.actor.direction = direction;

    // skip if client has not even been sent to anyone yet
    if !client.ready {
      return;
    }

    let packet = ServerPacket::ActorMove {
      ticket: id.to_string(),
      x,
      y,
      z,
      direction,
    };

    let area = self.areas.get(&client.actor.area_id).unwrap();

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
    assert_assets(
      &self.socket,
      self.config.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      [mug_texture_path, mug_animation_path].iter(),
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_script);

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
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
    assert_assets(
      &self.socket,
      self.config.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      [mug_texture_path, mug_animation_path].iter(),
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_script);

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
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
    assert_assets(
      &self.socket,
      self.config.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      [mug_texture_path, mug_animation_path].iter(),
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_script);

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
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

  pub fn prompt_player(&mut self, id: &str, character_limit: u16, default_text: Option<String>) {
    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_script);

      // reliability + id + type + u16 size
      let available_space = self.config.max_payload_size as u16 - 1 - 8 - 2 - 2 - 2;

      let character_limit = std::cmp::min(character_limit, available_space);

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::Prompt {
          character_limit,
          default_text,
        },
      );
    }
  }

  pub fn open_board(
    &mut self,
    player_id: &str,
    name: String,
    color: (u8, u8, u8),
    posts: Vec<BbsPost>,
  ) {
    use super::bbs_post::calc_size;
    use crate::helpers::iterators::IteratorHelper;
    use std::cell::RefCell;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    let start_depth = client.widget_tracker.get_board_count() as u8;
    client.widget_tracker.track_board(self.active_script);

    if posts.is_empty() {
      // logic below will send nothing if there's no posts,
      // we want to at least open the bbs if the post vec is empty
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::OpenBoard {
          current_depth: start_depth,
          name: name.clone(),
          color,
          posts: &[],
        },
      );
    }

    let max_payload_size = self.config.max_payload_size;
    let chunk_state: Rc<RefCell<(usize, Option<String>)>> = Rc::new(RefCell::new((0, None)));

    let calc_chunk_limit = |_| {
      // reliability + id + type
      let mut packet_size = 1 + 8 + 2;

      let borrowed_state = chunk_state.borrow();

      if borrowed_state.0 == 0 {
        packet_size += 2 + name.len();
        packet_size += 3; // color
      } else {
        packet_size += 2; // currentDepth + hasReference

        if let Some(last_id) = borrowed_state.1.as_ref() {
          packet_size += 2 + last_id.len(); // reference
        }
      }

      max_payload_size - packet_size
    };

    let calc_post_size = |post: &BbsPost| {
      let mut borrowed_state = chunk_state.borrow_mut();
      borrowed_state.0 += 1;
      borrowed_state.1 = Some(post.id.clone());
      calc_size(post)
    };

    let chunks = posts
      .into_iter()
      .pack_chunks_lossy(calc_chunk_limit, calc_post_size);

    let mut last_id = None;
    let current_depth = client.widget_tracker.get_board_count() as u8;

    for (i, mut chunk) in chunks.enumerate() {
      let mut ref_id = None;
      std::mem::swap(&mut ref_id, &mut last_id); // avoiding clone

      let packet = if i == 0 {
        ServerPacket::OpenBoard {
          current_depth: start_depth,
          name: name.clone(),
          color,
          posts: chunk.as_slice(),
        }
      } else {
        ServerPacket::AppendPosts {
          current_depth,
          reference: ref_id,
          posts: chunk.as_slice(),
        }
      };

      client
        .packet_shipper
        .send(&self.socket, Reliability::ReliableOrdered, &packet);

      last_id = chunk.pop().map(|post| post.id);
    }
  }

  pub fn prepend_posts(&mut self, player_id: &str, reference: Option<String>, posts: Vec<BbsPost>) {
    use super::bbs_post::calc_size;
    use crate::helpers::iterators::IteratorHelper;
    use std::cell::RefCell;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    let max_payload_size = self.config.max_payload_size;

    let last_id = Rc::new(RefCell::new(reference.clone()));

    let calc_chunk_limit = |_| {
      // reliability + id + type
      let mut packet_size = 1 + 8 + 2;
      packet_size += 2; // currentDepth + hasReference

      if let Some(last_id) = last_id.borrow().as_ref() {
        packet_size += 2 + last_id.len(); // reference
      }

      max_payload_size - packet_size
    };

    let calc_post_size = |post: &BbsPost| {
      *last_id.borrow_mut() = Some(post.id.clone());
      calc_size(post)
    };

    let chunks = posts
      .into_iter()
      .pack_chunks_lossy(calc_chunk_limit, calc_post_size);

    let mut last_id = reference;
    let current_depth = client.widget_tracker.get_board_count() as u8;

    for (i, mut chunk) in chunks.enumerate() {
      let mut ref_id = None;
      std::mem::swap(&mut ref_id, &mut last_id); // avoiding clone

      let packet = if i == 0 {
        ServerPacket::PrependPosts {
          current_depth,
          reference: ref_id,
          posts: chunk.as_slice(),
        }
      } else {
        ServerPacket::AppendPosts {
          current_depth,
          reference: ref_id,
          posts: chunk.as_slice(),
        }
      };

      client
        .packet_shipper
        .send(&self.socket, Reliability::ReliableOrdered, &packet);

      last_id = chunk.pop().map(|post| post.id);
    }
  }

  pub fn append_posts(&mut self, player_id: &str, reference: Option<String>, posts: Vec<BbsPost>) {
    use super::bbs_post::calc_size;
    use crate::helpers::iterators::IteratorHelper;
    use std::cell::RefCell;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    let max_payload_size = self.config.max_payload_size;

    let last_id = Rc::new(RefCell::new(reference.clone()));

    let calc_chunk_limit = |_| {
      // reliability + id + type
      let mut packet_size = 1 + 8 + 2;
      packet_size += 2; // currentDepth + hasReference

      if let Some(last_id) = last_id.borrow().as_ref() {
        packet_size += 2 + last_id.len(); // reference
      }

      max_payload_size - packet_size
    };

    let calc_post_size = |post: &BbsPost| {
      *last_id.borrow_mut() = Some(post.id.clone());
      calc_size(post)
    };

    let chunks = posts
      .into_iter()
      .pack_chunks_lossy(calc_chunk_limit, calc_post_size);

    let mut last_id = reference;
    let current_depth = client.widget_tracker.get_board_count() as u8;

    for mut chunk in chunks {
      let mut ref_id = None;
      std::mem::swap(&mut ref_id, &mut last_id); // avoiding clone

      let packet = ServerPacket::AppendPosts {
        current_depth,
        reference: ref_id,
        posts: chunk.as_slice(),
      };

      client
        .packet_shipper
        .send(&self.socket, Reliability::ReliableOrdered, &packet);

      last_id = chunk.pop().map(|post| post.id);
    }
  }

  pub fn remove_post(&mut self, player_id: &str, post_id: &str) {
    if let Some(client) = self.clients.get_mut(player_id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::RemovePost {
          current_depth: client.widget_tracker.get_board_count() as u8,
          id: post_id.to_string(),
        },
      );
    }
  }

  pub fn close_bbs(&mut self, player_id: &str) {
    if let Some(client) = self.clients.get_mut(player_id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::CloseBBS,
      );
    }
  }

  pub fn is_player_battling(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_battling;
    }

    false
  }

  pub fn initiate_pvp(&mut self, player_1_id: &str, player_2_id: &str) {
    use crate::helpers::use_public_ip;
    use multi_mut::HashMapMultiMut;

    let (client_1, client_2) =
      if let Some((client_1, client_2)) = self.clients.get_pair_mut(player_1_id, player_2_id) {
        (client_1, client_2)
      } else {
        return;
      };

    client_1.is_battling = true;
    client_2.is_battling = true;

    // todo: put these clients in slow mode

    let client_1_addr = use_public_ip(client_1.socket_address, self.config.public_ip);
    let client_2_addr = use_public_ip(client_2.socket_address, self.config.public_ip);

    client_1.packet_shipper.send(
      &self.socket,
      Reliability::ReliableOrdered,
      &ServerPacket::InitiatePvp {
        address: client_2_addr.to_string(),
      },
    );

    client_2.packet_shipper.send(
      &self.socket,
      Reliability::ReliableOrdered,
      &ServerPacket::InitiatePvp {
        address: client_1_addr.to_string(),
      },
    )
  }

  pub fn is_player_busy(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_busy();
    }

    true
  }

  pub fn get_player_data(&self, player_id: &str) -> Option<&PlayerData> {
    self
      .clients
      .get(player_id)
      .map(|client| &client.player_data)
  }

  pub fn set_player_health(&mut self, player_id: &str, health: u32) {
    if let Some(client) = self.clients.get_mut(player_id) {
      let max_health = client.player_data.max_health;

      client.player_data.health = health;

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::Health { health, max_health },
      );
    }
  }

  pub fn set_player_max_health(&mut self, player_id: &str, max_health: u32) {
    if let Some(client) = self.clients.get_mut(player_id) {
      let health = client.player_data.health;

      client.player_data.max_health = max_health;

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::Health { health, max_health },
      );
    }
  }

  pub fn set_player_emotion(&mut self, player_id: &str, emotion: u8) {
    if let Some(client) = self.clients.get_mut(player_id) {
      client.player_data.emotion = emotion;

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::Emotion { emotion },
      );
    }
  }

  pub fn set_player_money(&mut self, player_id: &str, money: u32) {
    if let Some(client) = self.clients.get_mut(player_id) {
      client.player_data.money = money;

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::Money { money },
      );
    }
  }

  #[allow(clippy::too_many_arguments)]
  pub fn transfer_player(
    &mut self,
    id: &str,
    area_id: &str,
    warp_in: bool,
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  ) {
    if self.areas.get(area_id).is_none() {
      // non existent area
      return;
    }

    let client = if let Some(client) = self.clients.get_mut(id) {
      client
    } else {
      return;
    };

    let previous_area = self.areas.get_mut(&client.actor.area_id).unwrap();
    client.warp_in = warp_in;
    client.warp_x = x;
    client.warp_y = y;
    client.warp_z = z;
    client.warp_direction = direction;

    if !previous_area
      .get_connected_players()
      .contains(&id.to_string())
    {
      // client has not been added to any area yet
      // assume client was transferred on initial connection by a plugin
      client.actor.area_id = area_id.to_string();
      return;
    }

    client.warp_area = area_id.to_string();

    let previous_area = self.areas.get_mut(&client.actor.area_id).unwrap();
    previous_area.remove_player(&id);

    broadcast_to_area(
      &self.socket,
      &mut self.clients,
      previous_area,
      Reliability::ReliableOrdered,
      ServerPacket::ActorDisconnected {
        ticket: id.to_string(),
        warp_out: warp_in,
      },
    );

    if warp_in {
      let client = self.clients.get_mut(id).unwrap();

      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::TransferWarp,
      );
    } else {
      self.complete_transfer(id)
    }
  }

  pub(super) fn complete_transfer(&mut self, player_id: &str) {
    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    client.packet_shipper.send(
      &self.socket,
      Reliability::ReliableOrdered,
      &ServerPacket::TransferStart,
    );

    let area_id = client.warp_area.clone();
    let area = self.areas.get_mut(&area_id).unwrap();
    let texture_path = client.actor.texture_path.clone();
    let animation_path = client.actor.animation_path.clone();

    assert_assets(
      &self.socket,
      self.config.max_payload_size,
      &self.assets,
      &mut self.clients,
      area.get_connected_players(),
      [texture_path.as_str(), animation_path.as_str()].iter(),
    );

    area.add_player(player_id.to_string());
    self.send_area(player_id, &area_id);

    let mut client = self.clients.get_mut(player_id).unwrap();

    client.actor.area_id = area_id.to_string();
    client.transferring = true;
    client.ready = false;

    client.packet_shipper.send(
      &self.socket,
      Reliability::ReliableOrdered,
      &ServerPacket::Teleport {
        warp: false,
        x: client.warp_x,
        y: client.warp_y,
        z: client.warp_z,
        direction: client.warp_direction,
      },
    );

    client.packet_shipper.send(
      &self.socket,
      Reliability::ReliableOrdered,
      &ServerPacket::TransferComplete {
        warp_in: client.warp_in,
        direction: client.warp_direction,
      },
    );
  }

  pub fn transfer_server(
    &mut self,
    id: &str,
    address: &str,
    port: u16,
    data: &str,
    warp_out: bool,
  ) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        Reliability::ReliableOrdered,
        &ServerPacket::TransferServer {
          address: address.to_string(),
          port,
          data: data.to_string(),
          warp_out,
        },
      );

      self.kick_player(id, "Transferred", false);
    }
  }

  pub fn kick_player(&mut self, id: &str, reason: &str, warp_out: bool) {
    if let Some(client) = self.clients.get(id) {
      self.kick_list.push(Boot {
        socket_address: client.socket_address,
        reason: reason.to_string(),
        warp_out,
      });
    }
  }

  pub(super) fn get_kick_list(&self) -> &Vec<Boot> {
    &self.kick_list
  }

  pub(super) fn clear_kick_list(&mut self) {
    self.kick_list.clear();
  }

  pub(super) fn add_client(
    &mut self,
    socket_address: std::net::SocketAddr,
    name: String,
  ) -> String {
    let area_id = String::from("default");
    let area = self.get_area_mut(&area_id).unwrap();
    let map = area.get_map();
    let (spawn_x, spawn_y, spawn_z) = map.get_spawn();
    let spawn_direction = map.get_spawn_direction();

    let client = Client::new(
      socket_address,
      name,
      area_id,
      spawn_x,
      spawn_y,
      spawn_z,
      spawn_direction,
      self.config.resend_budget,
    );

    let id = client.actor.id.clone();

    self.clients.insert(id.clone(), client);

    id
  }

  pub(super) fn store_player_assets(&mut self, player_id: &str) -> Option<(String, String)> {
    use super::asset;
    use super::client::find_longest_frame_length;
    use std::array::IntoIter;

    let client = self.clients.get_mut(player_id).unwrap();

    let texture_data = client.texture_buffer.clone();
    let animation_data = String::from_utf8_lossy(&client.animation_buffer).into_owned();
    let mugshot_texture_data = client.mugshot_texture_buffer.clone();
    let mugshot_animation_data =
      String::from_utf8_lossy(&client.mugshot_animation_buffer).into_owned();

    // reset buffers to store new data later
    client.texture_buffer.clear();
    client.animation_buffer.clear();
    client.mugshot_texture_buffer.clear();
    client.mugshot_animation_buffer.clear();

    let avatar_dimensions_limit = self.config.avatar_dimensions_limit;

    if find_longest_frame_length(&animation_data) > avatar_dimensions_limit {
      let reason = format!(
        "Avatar has frames larger than limit {}x{}",
        avatar_dimensions_limit, avatar_dimensions_limit
      );

      self.kick_player(player_id, &reason, true);

      return None;
    }

    let texture_path = asset::get_player_texture_path(player_id);
    let animation_path = asset::get_player_animation_path(player_id);
    let mugshot_texture_path = asset::get_player_mugshot_texture_path(player_id);
    let mugshot_animation_path = asset::get_player_mugshot_animation_path(player_id);

    let player_assets = [
      (texture_path.clone(), AssetData::Texture(texture_data)),
      (animation_path.clone(), AssetData::Text(animation_data)),
      (
        mugshot_texture_path,
        AssetData::Texture(mugshot_texture_data),
      ),
      (
        mugshot_animation_path,
        AssetData::Text(mugshot_animation_data),
      ),
    ];

    for (path, data) in IntoIter::new(player_assets) {
      self.set_asset(
        path,
        Asset {
          data,
          dependencies: Vec::new(),
          last_modified: 0,
          cachable: false,
        },
      );
    }

    Some((texture_path, animation_path))
  }

  pub(super) fn spawn_client(&mut self, player_id: &str) {
    let client = self.clients.get(player_id).unwrap();
    let area_id = client.actor.area_id.clone();
    let texture_path = client.actor.texture_path.clone();
    let animation_path = client.actor.animation_path.clone();

    let area = self.areas.get_mut(&area_id).unwrap();
    area.add_player(client.actor.id.clone());

    assert_assets(
      &self.socket,
      self.config.max_payload_size,
      &self.assets,
      &mut self.clients,
      area.get_connected_players(),
      [texture_path.as_str(), animation_path.as_str()].iter(),
    );

    self.send_area(player_id, &area_id);

    let client = self.clients.get_mut(player_id).unwrap();

    let packet = ServerPacket::Login {
      ticket: player_id.to_string(),
      warp_in: client.warp_in,
      spawn_x: client.warp_x,
      spawn_y: client.warp_y,
      spawn_z: client.warp_z,
      spawn_direction: client.warp_direction,
    };

    client
      .packet_shipper
      .send(&self.socket, Reliability::ReliableOrdered, &packet);
  }

  pub(super) fn connect_client(&mut self, player_id: &str) {
    let client = self.clients.get_mut(player_id).unwrap();

    client.packet_shipper.send(
      &self.socket,
      Reliability::ReliableOrdered,
      &ServerPacket::CompleteConnection,
    );
  }

  fn send_area(&mut self, player_id: &str, area_id: &str) {
    use super::asset::get_map_path;

    let area = self.areas.get(area_id).unwrap();

    let mut packets: Vec<ServerPacket> = Vec::new();
    let mut asset_paths: Vec<String> = area.get_required_assets().clone();

    // send map
    let map_path = get_map_path(area_id);
    asset_paths.push(map_path.clone());
    packets.push(ServerPacket::MapUpdate { map_path });

    // send clients
    for other_player_id in area.get_connected_players() {
      if other_player_id == player_id {
        continue;
      }

      let other_client = self.clients.get(other_player_id).unwrap();
      let actor = &other_client.actor;

      asset_paths.push(actor.texture_path.clone());
      asset_paths.push(actor.animation_path.clone());

      packets.push(actor.create_spawn_packet(actor.x, actor.y, actor.z, false));
    }

    // send bots
    for bot_id in area.get_connected_bots() {
      let bot = self.bots.get(bot_id).unwrap();

      asset_paths.push(bot.texture_path.clone());
      asset_paths.push(bot.animation_path.clone());

      packets.push(bot.create_spawn_packet(bot.x, bot.y, bot.z, false));
    }

    if let Some(custom_emotes_path) = &self.config.custom_emotes_path {
      asset_paths.push(custom_emotes_path.clone());
      packets.push(ServerPacket::CustomEmotesPath {
        asset_path: custom_emotes_path.clone(),
      });
    }

    // send asset_packets before anything else
    let asset_recievers = vec![player_id.to_string()];

    for asset_path in asset_paths {
      assert_asset(
        &self.socket,
        self.config.max_payload_size,
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
        .send(&self.socket, Reliability::ReliableOrdered, &packet);
    }
  }

  // handles first join and completed transfer
  pub(super) fn mark_client_ready(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.ready = true;
      client.transferring = false;

      let packet = client.actor.create_spawn_packet(
        client.warp_x,
        client.warp_y,
        client.warp_z,
        client.warp_in,
      );

      let area = self.areas.get_mut(&client.actor.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::ReliableOrdered,
        packet,
      );
    }
  }

  pub(super) fn remove_player(&mut self, id: &str, warp_out: bool) {
    use super::asset;
    if let Some(client) = self.clients.remove(id) {
      let remove_list = [
        asset::get_player_texture_path(id),
        asset::get_player_animation_path(id),
        asset::get_player_mugshot_animation_path(id),
        asset::get_player_mugshot_texture_path(id),
      ];

      for asset_path in remove_list.iter() {
        self.assets.remove(asset_path);
      }

      let area = self
        .areas
        .get_mut(&client.actor.area_id)
        .expect("Missing area for removed client");

      area.remove_player(&client.actor.id);

      let packet = ServerPacket::ActorDisconnected {
        ticket: id.to_string(),
        warp_out,
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

  pub fn get_bot(&self, id: &str) -> Option<&Actor> {
    self.bots.get(id)
  }

  pub fn add_bot(&mut self, bot: Actor) {
    if self.bots.contains_key(&bot.id) {
      println!("A bot with id \"{}\" already exists!", bot.id);
      return;
    }

    if self.clients.contains_key(&bot.id) {
      println!("A player with id \"{}\" exists, can't create bot!", bot.id);
      return;
    }

    if let Some(area) = self.areas.get_mut(&bot.area_id) {
      area.add_bot(bot.id.clone());

      let packet = bot.create_spawn_packet(bot.x, bot.y, bot.z, true);

      assert_assets(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        area.get_connected_players(),
        [bot.texture_path.as_str(), bot.animation_path.as_str()].iter(),
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

  pub fn remove_bot(&mut self, id: &str) {
    if let Some(bot) = self.bots.remove(id) {
      let area = self
        .areas
        .get_mut(&bot.area_id)
        .expect("Missing area for removed bot");

      area.remove_bot(&bot.id);

      let packet = ServerPacket::ActorDisconnected {
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

  pub fn set_bot_name(&mut self, id: &str, name: String) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.name = name.clone();

      let packet = ServerPacket::ActorSetName {
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
      let updated_direction = Direction::from_offset(x - bot.x, y - bot.y);

      if !matches!(updated_direction, Direction::None) {
        bot.direction = updated_direction;
      }

      bot.x = x;
      bot.y = y;
      bot.z = z;
      bot.current_animation = None;
    }
  }

  pub fn set_bot_direction(&mut self, id: &str, direction: Direction) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.direction = direction;
    }
  }

  pub fn set_bot_avatar(&mut self, id: &str, texture_path: String, animation_path: String) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.texture_path = texture_path.clone();
      bot.animation_path = animation_path.clone();

      let area = self.areas.get(&bot.area_id).unwrap();

      update_cached_clients(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        &texture_path,
      );

      update_cached_clients(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        &animation_path,
      );

      let packet = ServerPacket::ActorSetAvatar {
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

  pub fn set_bot_emote(&mut self, id: &str, emote_id: u8, use_custom_emotes: bool) {
    if let Some(bot) = self.bots.get(id) {
      let packet = ServerPacket::ActorEmote {
        ticket: id.to_string(),
        emote_id,
        use_custom_emotes,
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

  pub fn animate_bot(&mut self, id: &str, name: &str, loop_animation: bool) {
    if let Some(bot) = self.bots.get(id) {
      let area = self.areas.get(&bot.area_id).unwrap();

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        ServerPacket::ActorAnimate {
          ticket: id.to_string(),
          state: name.to_string(),
          loop_animation,
        },
      );
    }
  }

  pub fn animate_bot_properties(&mut self, id: &str, animation: Vec<KeyFrame>) {
    use super::actor_property_animation::ActorProperty;

    if let Some(bot) = self.bots.get_mut(id) {
      // store final values for new players
      let area = match self.areas.get(&bot.area_id) {
        Some(area) => area,
        None => return,
      };

      for keyframe in &animation {
        for (property, _) in &keyframe.property_steps {
          match property {
            ActorProperty::Animation(value) => bot.current_animation = Some(value.clone()),
            ActorProperty::X(value) => bot.x = *value,
            ActorProperty::Y(value) => bot.y = *value,
            ActorProperty::Z(value) => bot.z = *value,
            ActorProperty::ScaleX(value) => bot.scale_x = *value,
            ActorProperty::ScaleY(value) => bot.scale_y = *value,
            ActorProperty::Rotation(value) => bot.rotation = *value,
            ActorProperty::Direction(value) => bot.direction = *value,
            _ => {}
          }
        }
      }

      broadcast_actor_keyframes(
        &self.socket,
        &mut self.clients,
        area,
        self.config.max_payload_size,
        id,
        animation,
      );
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
        ServerPacket::ActorDisconnected {
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

      assert_assets(
        &self.socket,
        self.config.max_payload_size,
        &self.assets,
        &mut self.clients,
        area.get_connected_players(),
        [bot.texture_path.as_str(), bot.animation_path.as_str()].iter(),
      );

      broadcast_to_area(
        &self.socket,
        &mut self.clients,
        area,
        Reliability::Reliable,
        bot.create_spawn_packet(bot.x, bot.y, bot.z, warp_in),
      );
    }
  }

  pub fn message_server(&mut self, address: String, port: u16, data: Vec<u8>) {
    use crate::jobs::message_server::message_server;

    if let Ok(socket) = self.socket.try_clone() {
      message_server(socket, address, port, data);
    }
  }

  // ugly opengl like context storing
  // needed to correctly track message owners send without adding extra parameters
  // luckily not visible to plugin authors
  pub(super) fn set_active_script(&mut self, active_script: usize) {
    self.active_script = active_script;
  }

  pub(super) fn tick(&mut self) {
    self.resend_backed_up_packets();
    self.broadcast_bot_positions();
    self.broadcast_map_changes();
  }

  fn broadcast_bot_positions(&mut self) {
    for bot in self.bots.values() {
      let packet = ServerPacket::ActorMove {
        ticket: bot.id.clone(),
        x: bot.x,
        y: bot.y,
        z: bot.z,
        direction: bot.direction,
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

      if map.asset_is_stale() {
        let map_asset = map.generate_asset();

        self.assets.insert(map_path.clone(), map_asset);
        update_cached_clients(
          &self.socket,
          self.config.max_payload_size,
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

fn broadcast_actor_keyframes(
  socket: &UdpSocket,
  clients: &mut HashMap<String, Client>,
  area: &Area,
  max_payload_size: usize,
  id: &str,
  animation: Vec<KeyFrame>,
) {
  use super::actor_property_animation::ActorProperty;
  use crate::helpers::iterators::IteratorHelper;

  // reliability + reliability id + type + u16 size + id + tail + keyframe size
  let header_size = 1 + 8 + 2 + 2 + id.len() + 1 + 2;
  let remaining_size = max_payload_size - header_size;

  let measure = |keyframe: &KeyFrame| {
    // duration + step count
    let mut size = 4 + 2;

    for (property, _) in &keyframe.property_steps {
      // ease id + property id
      size += 1 + 1;

      // + value
      match property {
        ActorProperty::Animation(value) => size += 2 + value.len(),
        ActorProperty::Direction(_) => size += 1,
        _ => size += 4,
      }
    }

    size
  };

  let chunks = animation
    .into_iter()
    .pack_chunks_lossy(|_| remaining_size, measure);

  let mut last_chunk = None;

  for chunk in chunks {
    if let Some(chunk) = last_chunk {
      broadcast_to_area(
        socket,
        clients,
        area,
        Reliability::ReliableOrdered,
        ServerPacket::ActorPropertyKeyFrames {
          ticket: id.to_string(),
          tail: false,
          keyframes: chunk,
        },
      )
    }

    last_chunk = Some(chunk)
  }

  if let Some(chunk) = last_chunk {
    broadcast_to_area(
      socket,
      clients,
      area,
      Reliability::ReliableOrdered,
      ServerPacket::ActorPropertyKeyFrames {
        ticket: id.to_string(),
        tail: true,
        keyframes: chunk,
      },
    )
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
          client.packet_shipper.send(socket, reliability, &packet);
        }
      }
    }
  }

  // updating clients who have this asset
  if let Some(asset) = assets.get(asset_path) {
    let packets = create_asset_stream(max_payload_size, asset_path, &asset);

    for client in &mut clients_to_update {
      for packet in &packets {
        client.packet_shipper.send(socket, reliability, &packet);
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
          .send(socket, Reliability::ReliableOrdered, &packet);
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

    client.packet_shipper.send(socket, reliability, &packet);
  }
}

fn assert_assets<'a, I>(
  socket: &UdpSocket,
  max_payload_size: usize,
  assets: &HashMap<String, Asset>,
  clients: &mut HashMap<String, Client>,
  player_ids: &[String],
  asset_paths: I,
) where
  I: std::iter::Iterator<Item = &'a &'a str>,
{
  for asset_path in asset_paths {
    assert_asset(
      socket,
      max_payload_size,
      assets,
      clients,
      player_ids,
      asset_path,
    );
  }
}
