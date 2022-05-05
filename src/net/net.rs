use super::actor_property_animation::KeyFrame;
use super::asset_manager::AssetManager;
use super::boot::Boot;
use super::client::Client;
use super::map::Map;
use super::server::ServerConfig;
use super::{Actor, Area, Asset, AssetData, BbsPost, Direction, Item, PlayerData, ShopItem};
use crate::packets::{create_asset_stream, PacketOrchestrator, Reliability, ServerPacket};
use log::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Net {
  socket: Rc<UdpSocket>,
  packet_orchestrator: Rc<RefCell<PacketOrchestrator>>,
  config: Rc<ServerConfig>,
  areas: HashMap<String, Area>,
  clients: HashMap<String, Client>,
  bots: HashMap<String, Actor>,
  asset_manager: AssetManager,
  active_plugin: usize,
  kick_list: Vec<Boot>,
  items: HashMap<String, Item>,
}

impl Net {
  pub fn new(
    socket: Rc<UdpSocket>,
    packet_orchestrator: Rc<RefCell<PacketOrchestrator>>,
    config: Rc<ServerConfig>,
  ) -> Net {
    use super::asset::get_map_path;
    use std::fs::{read_dir, read_to_string};

    let mut asset_manager = AssetManager::new();
    asset_manager.load_assets_from_dir(std::path::Path::new("assets"));

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

        let map_path = get_map_path(&area_id);
        let map_asset = map.generate_asset();

        asset_manager.set_asset(map_path, map_asset);
        areas.insert(area_id.clone(), Area::new(area_id, map));
      }
    }

    if !default_area_provided {
      panic!("No default (default.tmx) area data found");
    }

    Net {
      socket,
      packet_orchestrator,
      config,
      areas,
      clients: HashMap::new(),
      bots: HashMap::new(),
      asset_manager,
      active_plugin: 0,
      kick_list: Vec::new(),
      items: HashMap::new(),
    }
  }

  pub fn get_asset(&self, path: &str) -> Option<&Asset> {
    self.asset_manager.get_asset(path)
  }

  pub fn set_asset(&mut self, path: String, asset: Asset) {
    self.asset_manager.set_asset(path.clone(), asset);

    update_cached_clients(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
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
    let mut map = map;

    if let Some(area) = self.areas.get_mut(&id) {
      area.set_map(map);
    } else {
      use super::asset::get_map_path;

      let map_path = get_map_path(&id);
      self.asset_manager.set_asset(map_path, map.generate_asset());
      self.areas.insert(id.clone(), Area::new(id, map));
    }
  }

  pub fn remove_area(&mut self, id: &str) {
    use super::asset::get_map_path;

    if id == "default" {
      print!("Can't delete default area!");
      return;
    }

    let map_path = get_map_path(id);
    self.asset_manager.remove_asset(&map_path);

    if let Some(area) = self.areas.remove(id) {
      let player_ids = area.get_connected_players();

      for player_id in player_ids {
        self.kick_player(player_id, "Area destroyed", true);
      }
    }
  }

  pub fn remove_asset(&mut self, path: &str) {
    self.asset_manager.remove_asset(path);
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
      ensure_asset(
        &mut *self.packet_orchestrator.borrow_mut(),
        self.config.max_payload_size,
        &self.asset_manager,
        &mut self.clients,
        area.get_connected_players(),
        asset_path,
      );

      area.require_asset(asset_path.to_string());
    }
  }

  pub fn play_sound(&mut self, area_id: &str, path: &str) {
    if let Some(area) = self.areas.get(area_id) {
      ensure_asset(
        &mut *self.packet_orchestrator.borrow_mut(),
        self.config.max_payload_size,
        &self.asset_manager,
        &mut self.clients,
        area.get_connected_players(),
        path,
      );

      broadcast_to_area(
        &mut *self.packet_orchestrator.borrow_mut(),
        area,
        Reliability::Reliable,
        ServerPacket::PlaySound { path },
      )
    }
  }

  pub fn set_player_name(&mut self, id: &str, name: &str) {
    let client = match self.clients.get_mut(id) {
      Some(client) => client,
      None => return,
    };

    client.actor.name = name.to_string();

    if !client.ready {
      // skip if client has not even been sent to anyone yet
      return;
    }

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    let packet = ServerPacket::ActorSetName { ticket: id, name };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      packet,
    );
  }

  pub fn set_player_avatar(&mut self, id: &str, texture_path: &str, animation_path: &str) {
    let client = match self.clients.get_mut(id) {
      Some(client) => client,
      None => return,
    };

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    client.actor.texture_path = texture_path.to_string();
    client.actor.animation_path = animation_path.to_string();

    // we'd normally skip if the player has not been sent to anyone yet
    // but for this we want to make sure the player sees this and updates their avatar
    // if the other players receive this, they'll just ignore it

    ensure_assets(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      area.get_connected_players(),
      [texture_path, animation_path].iter(),
    );

    let packet = ServerPacket::ActorSetAvatar {
      ticket: id,
      texture_path,
      animation_path,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::ReliableOrdered,
      packet,
    );
  }

  pub fn set_player_emote(&mut self, id: &str, emote_id: u8, use_custom_emotes: bool) {
    let client = match self.clients.get(id) {
      Some(client) => client,
      None => return,
    };

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    let packet = ServerPacket::ActorEmote {
      ticket: id,
      emote_id,
      use_custom_emotes,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      packet,
    );
  }

  pub fn exclusive_player_emote(
    &mut self,
    target_id: &str,
    emoter_id: &str,
    emote_id: u8,
    use_custom_emotes: bool,
  ) {
    let packet = ServerPacket::ActorEmote {
      ticket: emoter_id,
      emote_id,
      use_custom_emotes,
    };

    self
      .packet_orchestrator
      .borrow_mut()
      .send_by_id(target_id, Reliability::Reliable, packet);
  }

  pub fn set_player_minimap_color(&mut self, id: &str, color: (u8, u8, u8, u8)) {
    let client = match self.clients.get_mut(id) {
      Some(client) => client,
      None => return,
    };

    client.actor.minimap_color = color;

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      ServerPacket::ActorMinimapColor { ticket: id, color },
    );
  }

  pub fn animate_player(&mut self, id: &str, name: &str, loop_animation: bool) {
    let client = match self.clients.get(id) {
      Some(client) => client,
      None => return,
    };

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      ServerPacket::ActorAnimate {
        ticket: id,
        state: name,
        loop_animation,
      },
    );
  }

  pub fn animate_player_properties(&mut self, id: &str, animation: Vec<KeyFrame>) {
    use super::actor_property_animation::ActorProperty;
    use std::collections::HashSet;

    let client = match self.clients.get_mut(id) {
      Some(client) => client,
      None => return,
    };

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
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

    ensure_assets(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[id.to_string()],
      asset_paths.iter(),
    );

    broadcast_actor_keyframes(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      self.config.max_payload_size,
      id,
      animation,
    );
  }

  pub fn is_player_in_widget(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_in_widget();
    }

    false
  }

  pub fn is_player_shopping(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_shopping();
    }

    false
  }

  pub fn preload_asset_for_player(&mut self, id: &str, asset_path: &str) {
    ensure_asset(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[String::from(id)],
      asset_path,
    );

    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::Preload { asset_path },
    );
  }

  pub fn play_sound_for_player(&mut self, id: &str, path: &str) {
    ensure_asset(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[id.to_string()],
      path,
    );

    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::PlaySound { path },
    );
  }

  pub fn exclude_object_for_player(&mut self, id: &str, object_id: u32) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::ExcludeObject { id: object_id },
    );
  }

  pub fn include_object_for_player(&mut self, id: &str, object_id: u32) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::IncludeObject { id: object_id },
    );
  }

  pub fn exclude_actor_for_player(&mut self, id: &str, actor_id: &str) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::ExcludeActor { actor_id },
    );
  }

  pub fn include_actor_for_player(&mut self, id: &str, actor_id: &str) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::IncludeActor { actor_id },
    );
  }

  pub fn move_player_camera(&mut self, id: &str, x: f32, y: f32, z: f32, hold_time: f32) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::MoveCamera { x, y, z, hold_time },
    );
  }

  pub fn slide_player_camera(&mut self, id: &str, x: f32, y: f32, z: f32, duration: f32) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::SlideCamera { x, y, z, duration },
    );
  }

  pub fn shake_player_camera(&mut self, id: &str, strength: f32, duration: f32) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::ShakeCamera { strength, duration },
    );
  }

  pub fn fade_player_camera(&mut self, id: &str, color: (u8, u8, u8, u8), duration: f32) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::FadeCamera { duration, color },
    );
  }

  pub fn track_with_player_camera(&mut self, id: &str, actor_id: Option<&str>) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::TrackWithCamera { actor_id },
    );
  }

  pub fn unlock_player_camera(&mut self, id: &str) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::UnlockCamera,
    );
  }

  pub fn is_player_input_locked(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_input_locked;
    }

    false
  }

  pub fn lock_player_input(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.is_input_locked = true;

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::LockInput,
      );
    }
  }

  pub fn unlock_player_input(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.is_input_locked = false;

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::UnlockInput,
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
      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Teleport {
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

    client.actor.set_position(x, y, z);
    client.actor.set_direction(direction);

    // skip if client has not even been sent to anyone yet
    if !client.ready {
      return;
    }

    let idle_duration = client.actor.last_movement_time.elapsed().as_secs_f32();

    // skip if we've been sending idle packets for too long
    if idle_duration > self.config.max_idle_packet_duration {
      return;
    }

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    let packet = ServerPacket::ActorMove {
      ticket: id,
      x,
      y,
      z,
      direction,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
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
    ensure_assets(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[id.to_string()],
      [mug_texture_path, mug_animation_path].iter(),
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_plugin);

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Message {
          message,
          mug_texture_path,
          mug_animation_path,
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
    ensure_assets(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[id.to_string()],
      [mug_texture_path, mug_animation_path].iter(),
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_plugin);

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Question {
          message,
          mug_texture_path,
          mug_animation_path,
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
    ensure_assets(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[id.to_string()],
      [mug_texture_path, mug_animation_path].iter(),
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_plugin);

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Quiz {
          option_a,
          option_b,
          option_c,
          mug_texture_path,
          mug_animation_path,
        },
      );
    }
  }

  pub fn prompt_player(&mut self, id: &str, character_limit: u16, default_text: Option<&str>) {
    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_plugin);

      // reliability + id + type + u16 size
      let available_space = self.config.max_payload_size as u16 - 1 - 8 - 2 - 2 - 2;

      let character_limit = std::cmp::min(character_limit, available_space);

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Prompt {
          character_limit,
          default_text,
        },
      );
    }
  }

  pub fn open_board(
    &mut self,
    player_id: &str,
    name: &str,
    color: (u8, u8, u8),
    posts: Vec<BbsPost>,
  ) {
    use super::bbs_post::calc_size;
    use crate::helpers::iterators::IteratorHelper;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    let mut packet_orchestrator = self.packet_orchestrator.borrow_mut();

    let start_depth = client.widget_tracker.get_board_count() as u8;
    client.widget_tracker.track_board(self.active_plugin);

    if posts.is_empty() {
      // logic below will send nothing if there's no posts,
      // we want to at least open the bbs if the post vec is empty
      packet_orchestrator.send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::OpenBoard {
          current_depth: start_depth,
          name,
          color,
          posts: &[],
        },
      );
    }

    let max_payload_size = self.config.max_payload_size;
    let chunk_state: RefCell<(usize, Option<String>)> = RefCell::new((0, None));

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

    use crate::packets::build_packet;

    let mut packets = Vec::new();

    for (i, mut chunk) in chunks.enumerate() {
      let mut ref_id = None;
      std::mem::swap(&mut ref_id, &mut last_id); // avoiding clone

      let packet = if i == 0 {
        ServerPacket::OpenBoard {
          current_depth: start_depth,
          name,
          color,
          posts: chunk.as_slice(),
        }
      } else {
        ServerPacket::AppendPosts {
          current_depth,
          reference: ref_id.as_deref(),
          posts: chunk.as_slice(),
        }
      };

      packets.push(build_packet(packet));

      last_id = chunk.pop().map(|post| post.id);
    }

    packet_orchestrator.send_byte_packets(
      client.socket_address,
      Reliability::ReliableOrdered,
      &packets,
    );
  }

  pub fn prepend_posts(&mut self, player_id: &str, reference: Option<&str>, posts: Vec<BbsPost>) {
    use super::bbs_post::calc_size;
    use crate::helpers::iterators::IteratorHelper;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    let max_payload_size = self.config.max_payload_size;

    let reference = reference.map(|reference_str| reference_str.to_string());
    let last_id = RefCell::new(reference.clone());

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

    use crate::packets::build_packet;

    let mut packets = Vec::new();

    for (i, mut chunk) in chunks.enumerate() {
      let mut ref_id = None;
      std::mem::swap(&mut ref_id, &mut last_id); // avoiding clone

      let packet = if i == 0 {
        ServerPacket::PrependPosts {
          current_depth,
          reference: ref_id.as_deref(),
          posts: chunk.as_slice(),
        }
      } else {
        ServerPacket::AppendPosts {
          current_depth,
          reference: ref_id.as_deref(),
          posts: chunk.as_slice(),
        }
      };

      packets.push(build_packet(packet));

      last_id = chunk.pop().map(|post| post.id);
    }

    self.packet_orchestrator.borrow_mut().send_byte_packets(
      client.socket_address,
      Reliability::ReliableOrdered,
      &packets,
    );
  }

  pub fn append_posts(&mut self, player_id: &str, reference: Option<&str>, posts: Vec<BbsPost>) {
    use super::bbs_post::calc_size;
    use crate::helpers::iterators::IteratorHelper;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    let max_payload_size = self.config.max_payload_size;

    let reference = reference.map(|reference_str| reference_str.to_string());
    let last_id = RefCell::new(reference.clone());

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

    use crate::packets::build_packet;

    let mut packets = Vec::new();

    for mut chunk in chunks {
      let mut ref_id = None;
      std::mem::swap(&mut ref_id, &mut last_id); // avoiding clone

      let packet = ServerPacket::AppendPosts {
        current_depth,
        reference: ref_id.as_deref(),
        posts: chunk.as_slice(),
      };

      packets.push(build_packet(packet));

      last_id = chunk.pop().map(|post| post.id);
    }

    self.packet_orchestrator.borrow_mut().send_byte_packets(
      client.socket_address,
      Reliability::ReliableOrdered,
      &packets,
    );
  }

  pub fn remove_post(&mut self, player_id: &str, post_id: &str) {
    if let Some(client) = self.clients.get(player_id) {
      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::RemovePost {
          current_depth: client.widget_tracker.get_board_count() as u8,
          id: post_id,
        },
      );
    }
  }

  pub fn close_bbs(&mut self, player_id: &str) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      player_id,
      Reliability::ReliableOrdered,
      ServerPacket::CloseBBS,
    );
  }

  pub fn open_shop(
    &mut self,
    player_id: &str,
    items: Vec<ShopItem>,
    mug_texture_path: &str,
    mug_animation_path: &str,
  ) {
    use super::shop_item::calc_size;
    use crate::helpers::iterators::IteratorHelper;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    client.widget_tracker.track_shop(self.active_plugin);

    let max_payload_size = self.config.max_payload_size;

    let calc_chunk_limit = |_| {
      // reliability + id + type
      let header_size = 1 + 8 + 2;

      max_payload_size - header_size
    };

    use crate::packets::build_packet;

    let mut packets: Vec<Vec<u8>> = items
      .into_iter()
      .pack_chunks_lossy(calc_chunk_limit, calc_size)
      .map(|chunk| {
        build_packet(ServerPacket::ShopInventory {
          items: chunk.as_slice(),
        })
      })
      .collect();

    packets.push(build_packet(ServerPacket::OpenShop {
      mug_texture_path,
      mug_animation_path,
    }));

    self.packet_orchestrator.borrow_mut().send_byte_packets(
      client.socket_address,
      Reliability::ReliableOrdered,
      &packets,
    );
  }

  pub fn is_player_battling(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_battling();
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

    client_1.battle_tracker.push_back(self.active_plugin);
    client_2.battle_tracker.push_back(self.active_plugin);

    // todo: put these clients in slow mode

    let client_1_addr = use_public_ip(client_1.socket_address, self.config.public_ip);
    let client_2_addr = use_public_ip(client_2.socket_address, self.config.public_ip);

    let mut orchestrator = self.packet_orchestrator.borrow_mut();

    orchestrator.send(
      client_1.socket_address,
      Reliability::ReliableOrdered,
      ServerPacket::InitiatePvp {
        address: &client_2_addr.to_string(),
      },
    );

    orchestrator.send(
      client_2.socket_address,
      Reliability::ReliableOrdered,
      ServerPacket::InitiatePvp {
        address: &client_1_addr.to_string(),
      },
    );
  }

  fn send_encounter_data_asset(&mut self, player_id: &str, data: Option<&str>) {
    use super::asset;

    // store data as an asset
    let data_path = asset::get_encounter_data_path();
    let data_path_buf = std::path::PathBuf::from(data_path.to_string());
    let mut data_asset =
      Asset::load_from_memory(&data_path_buf, data.unwrap_or_default().as_bytes());
    data_asset.cachable = false;
    data_asset.cache_to_disk = false;

    self
      .asset_manager
      .set_asset(data_path.to_string(), data_asset);

    ensure_asset(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[player_id.to_string()],
      data_path,
    );
  }

  pub fn set_mod_whitelist_for_player(&mut self, player_id: &str, whitelist_path: &str) {
    ensure_asset(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[String::from(player_id)],
      &whitelist_path.to_string(),
    );

    self.packet_orchestrator.borrow_mut().send_by_id(
      player_id,
      Reliability::ReliableOrdered,
      ServerPacket::ModWhitelist { whitelist_path },
    );
  }

  pub fn offer_package(&mut self, player_id: &str, package_path: &str) {
    ensure_asset(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[String::from(player_id)],
      &package_path.to_string(),
    );

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    // send dependencies
    let dependency_chain = self
      .asset_manager
      .get_flattened_dependency_chain(package_path);

    let mut packets = Vec::new();
    packets.reserve_exact(dependency_chain.len());

    for asset_path in dependency_chain {
      let asset = if let Some(asset) = self.asset_manager.get_asset(asset_path) {
        asset
      } else {
        warn!("No asset found with path \"{}\"", asset_path);
        continue;
      };

      let package_info = if let Some(package_info) = asset.resolve_package_info() {
        package_info
      } else {
        warn!("\"{}\" is not a package", asset_path);
        continue;
      };

      packets.push(ServerPacket::OfferPackage {
        package_info,
        package_path: asset_path,
      });
    }

    self.packet_orchestrator.borrow_mut().send_packets(
      client.socket_address,
      Reliability::ReliableOrdered,
      packets,
    );
  }

  pub fn initiate_encounter(&mut self, player_id: &str, package_path: &str, data: Option<&str>) {
    use super::asset;

    ensure_asset(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      &[String::from(player_id)],
      &package_path.to_string(),
    );

    self.send_encounter_data_asset(player_id, data);

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    // update tracking
    client.battle_tracker.push_back(self.active_plugin);

    // send dependencies
    let dependency_chain = self
      .asset_manager
      .get_flattened_dependency_chain(package_path);

    let mut packets = Vec::new();
    packets.reserve_exact(dependency_chain.len());

    for asset_path in dependency_chain {
      let asset = if let Some(asset) = self.asset_manager.get_asset(asset_path) {
        asset
      } else {
        warn!("No asset found with path \"{}\"", asset_path);
        continue;
      };

      let package_category = if let Some(package_info) = asset.resolve_package_info() {
        package_info.category
      } else {
        warn!("\"{}\" is not a package", asset_path);
        continue;
      };

      packets.push(ServerPacket::LoadPackage {
        package_path: asset_path,
        package_category,
      });
    }

    packets.push(ServerPacket::InitiateEncounter {
      package_path,
      data_path: asset::get_encounter_data_path(),
    });

    self.packet_orchestrator.borrow_mut().send_packets(
      client.socket_address,
      Reliability::ReliableOrdered,
      packets,
    );
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

  pub(crate) fn update_player_data(&mut self, player_id: &str, element: String, max_health: u32) {
    let client = self.clients.get_mut(player_id).unwrap();

    client.player_data.element = element;
    client.player_data.health = max_health;
    client.player_data.max_health = max_health;
  }

  pub fn set_player_health(&mut self, player_id: &str, health: u32) {
    if let Some(client) = self.clients.get_mut(player_id) {
      let max_health = client.player_data.max_health;

      client.player_data.health = health;

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Health { health, max_health },
      );
    }
  }

  pub fn set_player_max_health(&mut self, player_id: &str, max_health: u32) {
    if let Some(client) = self.clients.get_mut(player_id) {
      let health = client.player_data.health;

      client.player_data.max_health = max_health;

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Health { health, max_health },
      );
    }
  }

  pub fn set_player_emotion(&mut self, player_id: &str, emotion: u8) {
    if let Some(client) = self.clients.get_mut(player_id) {
      client.player_data.emotion = emotion;

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Emotion { emotion },
      );
    }
  }

  pub fn set_player_money(&mut self, player_id: &str, money: u32) {
    if let Some(client) = self.clients.get_mut(player_id) {
      client.player_data.money = money;

      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::Money { money },
      );
    }
  }

  pub fn get_item(&mut self, item_id: &str) -> Option<&Item> {
    self.items.get(item_id)
  }

  pub fn set_item(&mut self, item_id: String, item: Item) {
    self.items.insert(item_id, item);
  }

  pub fn give_player_item(&mut self, player_id: &str, item_id: String) {
    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    let item = if let Some(item) = self.items.get(&item_id) {
      item
    } else {
      warn!("No item found with id \"{}\"", item_id);
      return;
    };

    self.packet_orchestrator.borrow_mut().send(
      client.socket_address,
      Reliability::ReliableOrdered,
      ServerPacket::AddItem {
        id: &item_id,
        name: &item.name,
        description: &item.description,
      },
    );

    client.player_data.items.push(item_id);
  }

  pub fn remove_player_item(&mut self, player_id: &str, item_id: &str) {
    if let Some(client) = self.clients.get_mut(player_id) {
      self.packet_orchestrator.borrow_mut().send(
        client.socket_address,
        Reliability::ReliableOrdered,
        ServerPacket::RemoveItem { id: item_id },
      );

      let items = &mut client.player_data.items;

      if let Some(index) = items.iter().position(|item| *item == item_id) {
        items.remove(index);
      }
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

    let client = match self.clients.get_mut(id) {
      Some(client) => client,
      None => return,
    };

    let previous_area = match self.areas.get_mut(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

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

    previous_area.remove_player(id);

    self
      .packet_orchestrator
      .borrow_mut()
      .leave_room(client.socket_address, previous_area.get_id());

    client.warp_area = area_id.to_string();

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      previous_area,
      Reliability::ReliableOrdered,
      ServerPacket::ActorDisconnected {
        ticket: id,
        warp_out: warp_in,
      },
    );

    if warp_in {
      self.packet_orchestrator.borrow_mut().send_by_id(
        id,
        Reliability::ReliableOrdered,
        ServerPacket::TransferWarp,
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

    self.packet_orchestrator.borrow_mut().send(
      client.socket_address,
      Reliability::ReliableOrdered,
      ServerPacket::TransferStart,
    );

    let area_id = client.warp_area.clone();
    let area = if let Some(area) = self.areas.get_mut(&area_id) {
      area
    } else {
      self.kick_player(player_id, "Area destroyed", true);
      return;
    };

    let texture_path = client.actor.texture_path.clone();
    let animation_path = client.actor.animation_path.clone();

    self
      .packet_orchestrator
      .borrow_mut()
      .join_room(client.socket_address, area_id.clone());

    ensure_assets(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
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

    self.packet_orchestrator.borrow_mut().send(
      client.socket_address,
      Reliability::ReliableOrdered,
      ServerPacket::Teleport {
        warp: false,
        x: client.warp_x,
        y: client.warp_y,
        z: client.warp_z,
        direction: client.warp_direction,
      },
    );

    self.packet_orchestrator.borrow_mut().send(
      client.socket_address,
      Reliability::ReliableOrdered,
      ServerPacket::TransferComplete {
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
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::TransferServer {
        address,
        port,
        data,
        warp_out,
      },
    );

    self.kick_player(id, "Transferred", warp_out);
  }

  pub fn request_authorization(&mut self, id: &str, address: &str, port: u16, data: &[u8]) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      id,
      Reliability::ReliableOrdered,
      ServerPacket::Authorize {
        address,
        port,
        data,
      },
    );
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

  pub(super) fn take_kick_list(&mut self) -> Vec<Boot> {
    let mut out = Vec::new();

    std::mem::swap(&mut self.kick_list, &mut out);

    out
  }

  pub(super) fn add_client(
    &mut self,
    socket_address: std::net::SocketAddr,
    name: String,
    identity: String,
  ) -> String {
    let area_id = String::from("default");
    let area = self.get_area_mut(&area_id).unwrap();
    let map = area.get_map();
    let (spawn_x, spawn_y, spawn_z) = map.get_spawn();
    let spawn_direction = map.get_spawn_direction();

    let client = Client::new(
      socket_address,
      name,
      identity,
      area_id.clone(),
      spawn_x,
      spawn_y,
      spawn_z,
      spawn_direction,
    );

    let id = client.actor.id.clone();

    let mut packet_orchestrator = self.packet_orchestrator.borrow_mut();
    packet_orchestrator.add_client(client.socket_address, id.clone());

    self.clients.insert(id.clone(), client);

    id
  }

  pub(super) fn store_player_assets(&mut self, player_id: &str) -> Option<(String, String)> {
    use super::asset;
    use super::client::find_longest_frame_length;

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

    for (path, data) in player_assets.into_iter() {
      self.set_asset(
        path,
        Asset {
          data,
          alternate_names: Vec::new(),
          dependencies: Vec::new(),
          last_modified: 0,
          cachable: true,
          cache_to_disk: false,
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

    let area = match self.areas.get_mut(&area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    area.add_player(client.actor.id.clone());

    self
      .packet_orchestrator
      .borrow_mut()
      .join_room(client.socket_address, area_id.clone());

    ensure_assets(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      area.get_connected_players(),
      [texture_path.as_str(), animation_path.as_str()].iter(),
    );

    self.send_area(player_id, &area_id);

    let client = self.clients.get_mut(player_id).unwrap();

    let packet = ServerPacket::Login {
      ticket: player_id,
      warp_in: client.warp_in,
      spawn_x: client.warp_x,
      spawn_y: client.warp_y,
      spawn_z: client.warp_z,
      spawn_direction: client.warp_direction,
    };

    self.packet_orchestrator.borrow_mut().send(
      client.socket_address,
      Reliability::ReliableOrdered,
      packet,
    );
  }

  pub(super) fn connect_client(&mut self, player_id: &str) {
    self.packet_orchestrator.borrow_mut().send_by_id(
      player_id,
      Reliability::ReliableOrdered,
      ServerPacket::CompleteConnection,
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
    packets.push(ServerPacket::MapUpdate {
      map_path: &map_path,
    });

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
        asset_path: custom_emotes_path,
      });
    }

    // build and collect packets to avoid lifetime overlap
    use crate::packets::build_packet;
    let packets: Vec<Vec<u8>> = packets.into_iter().map(build_packet).collect();

    // send asset_packets before anything else
    let asset_recievers = vec![player_id.to_string()];

    for asset_path in asset_paths {
      ensure_asset(
        &mut *self.packet_orchestrator.borrow_mut(),
        self.config.max_payload_size,
        &self.asset_manager,
        &mut self.clients,
        &asset_recievers[..],
        &asset_path,
      );
    }

    self
      .packet_orchestrator
      .borrow_mut()
      .send_byte_packets_by_id(player_id, Reliability::ReliableOrdered, &packets);
  }

  // handles first join and completed transfer
  pub(super) fn mark_client_ready(&mut self, id: &str) {
    use crate::packets::build_packet;

    let client = match self.clients.get_mut(id) {
      Some(client) => client,
      None => return,
    };

    let area = match self.areas.get(&client.actor.area_id) {
      Some(area) => area,
      None => return, // area deleted, should be getting kicked
    };

    client.ready = true;
    client.transferring = false;

    let packet =
      client
        .actor
        .create_spawn_packet(client.warp_x, client.warp_y, client.warp_z, client.warp_in);

    let packet_bytes = build_packet(packet);

    self
      .packet_orchestrator
      .borrow_mut()
      .broadcast_bytes_to_room(area.get_id(), Reliability::ReliableOrdered, packet_bytes);
  }

  pub(super) fn remove_player(&mut self, id: &str, warp_out: bool) {
    use super::asset;

    let client = match self.clients.remove(id) {
      Some(client) => client,
      None => return,
    };

    let remove_list = [
      asset::get_player_texture_path(id),
      asset::get_player_animation_path(id),
      asset::get_player_mugshot_animation_path(id),
      asset::get_player_mugshot_texture_path(id),
    ];

    for asset_path in remove_list.iter() {
      self.asset_manager.remove_asset(asset_path);
    }

    let area = match self.areas.get_mut(&client.actor.area_id) {
      Some(area) => area,
      None => return,
    };

    area.remove_player(&client.actor.id);

    let packet = ServerPacket::ActorDisconnected {
      ticket: id,
      warp_out,
    };

    let mut packet_orchestrator = self.packet_orchestrator.borrow_mut();

    broadcast_to_area(
      &mut packet_orchestrator,
      area,
      Reliability::ReliableOrdered,
      packet,
    );

    packet_orchestrator.drop_client(client.socket_address);
  }

  pub fn get_bot(&self, id: &str) -> Option<&Actor> {
    self.bots.get(id)
  }

  pub fn add_bot(&mut self, bot: Actor, warp_in: bool) {
    if self.bots.contains_key(&bot.id) {
      warn!("A bot with id \"{}\" already exists!", bot.id);
      return;
    }

    if self.clients.contains_key(&bot.id) {
      warn!("A player with id \"{}\" exists, can't create bot!", bot.id);
      return;
    }

    if let Some(area) = self.areas.get_mut(&bot.area_id) {
      area.add_bot(bot.id.clone());

      let packet = bot.create_spawn_packet(bot.x, bot.y, bot.z, warp_in);

      ensure_assets(
        &mut *self.packet_orchestrator.borrow_mut(),
        self.config.max_payload_size,
        &self.asset_manager,
        &mut self.clients,
        area.get_connected_players(),
        [bot.texture_path.as_str(), bot.animation_path.as_str()].iter(),
      );

      broadcast_to_area(
        &mut *self.packet_orchestrator.borrow_mut(),
        area,
        Reliability::ReliableOrdered,
        packet,
      );

      self.bots.insert(bot.id.clone(), bot);
    }
  }

  pub fn remove_bot(&mut self, id: &str, warp_out: bool) {
    let bot = match self.bots.remove(id) {
      Some(bot) => bot,
      None => return,
    };

    let area = match self.areas.get_mut(&bot.area_id) {
      Some(area) => area,
      None => return,
    };

    area.remove_bot(&bot.id);

    let packet = ServerPacket::ActorDisconnected {
      ticket: id,
      warp_out,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::ReliableOrdered,
      packet,
    );
  }

  pub fn set_bot_name(&mut self, id: &str, name: &str) {
    let bot = match self.bots.get_mut(id) {
      Some(bot) => bot,
      None => return,
    };

    bot.name = name.to_string();

    let area = match self.areas.get(&bot.area_id) {
      Some(area) => area,
      None => return,
    };

    let packet = ServerPacket::ActorSetName { ticket: id, name };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      packet,
    );
  }

  pub fn move_bot(&mut self, id: &str, x: f32, y: f32, z: f32) {
    if let Some(bot) = self.bots.get_mut(id) {
      let updated_direction = Direction::from_offset(x - bot.x, y - bot.y);

      if !matches!(updated_direction, Direction::None) {
        bot.set_direction(updated_direction);
      }

      bot.set_position(x, y, z);
    }
  }

  pub fn set_bot_direction(&mut self, id: &str, direction: Direction) {
    if let Some(bot) = self.bots.get_mut(id) {
      bot.set_direction(direction);
    }
  }

  pub fn set_bot_avatar(&mut self, id: &str, texture_path: &str, animation_path: &str) {
    let bot = match self.bots.get_mut(id) {
      Some(bot) => bot,
      None => return,
    };

    bot.texture_path = texture_path.to_string();
    bot.animation_path = animation_path.to_string();

    update_cached_clients(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      texture_path,
    );

    update_cached_clients(
      &mut *self.packet_orchestrator.borrow_mut(),
      self.config.max_payload_size,
      &self.asset_manager,
      &mut self.clients,
      animation_path,
    );

    let area = match self.areas.get(&bot.area_id) {
      Some(area) => area,
      None => return,
    };

    let packet = ServerPacket::ActorSetAvatar {
      ticket: id,
      texture_path,
      animation_path,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::ReliableOrdered,
      packet,
    );
  }

  pub fn set_bot_emote(&mut self, id: &str, emote_id: u8, use_custom_emotes: bool) {
    let bot = match self.bots.get(id) {
      Some(bot) => bot,
      None => return,
    };

    let area = match self.areas.get(&bot.area_id) {
      Some(area) => area,
      None => return,
    };

    let packet = ServerPacket::ActorEmote {
      ticket: id,
      emote_id,
      use_custom_emotes,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      packet,
    );
  }

  pub fn set_bot_minimap_color(&mut self, id: &str, color: (u8, u8, u8, u8)) {
    let bot = match self.bots.get_mut(id) {
      Some(bot) => bot,
      None => return,
    };

    bot.minimap_color = color;

    let area = match self.areas.get(&bot.area_id) {
      Some(area) => area,
      None => return,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      ServerPacket::ActorMinimapColor { ticket: id, color },
    );
  }

  pub fn animate_bot(&mut self, id: &str, name: &str, loop_animation: bool) {
    let bot = match self.bots.get(id) {
      Some(bot) => bot,
      None => return,
    };

    let area = match self.areas.get(&bot.area_id) {
      Some(area) => area,
      None => return,
    };

    broadcast_to_area(
      &mut *self.packet_orchestrator.borrow_mut(),
      area,
      Reliability::Reliable,
      ServerPacket::ActorAnimate {
        ticket: id,
        state: name,
        loop_animation,
      },
    );
  }

  pub fn animate_bot_properties(&mut self, id: &str, animation: Vec<KeyFrame>) {
    use super::actor_property_animation::ActorProperty;

    if let Some(bot) = self.bots.get_mut(id) {
      // store final values for new players
      let area = match self.areas.get(&bot.area_id) {
        Some(area) => area,
        None => return,
      };

      let mut final_x = bot.x;
      let mut final_y = bot.y;
      let mut final_z = bot.z;

      for keyframe in &animation {
        for (property, _) in &keyframe.property_steps {
          match property {
            ActorProperty::Animation(value) => bot.current_animation = Some(value.clone()),
            ActorProperty::X(value) => final_x = *value,
            ActorProperty::Y(value) => final_y = *value,
            ActorProperty::Z(value) => final_z = *value,
            ActorProperty::ScaleX(value) => bot.scale_x = *value,
            ActorProperty::ScaleY(value) => bot.scale_y = *value,
            ActorProperty::Rotation(value) => bot.rotation = *value,
            ActorProperty::Direction(value) => bot.direction = *value,
            _ => {}
          }
        }
      }

      // set position directly, to avoid reseting the animation
      bot.x = final_x;
      bot.y = final_y;
      bot.z = final_z;

      broadcast_actor_keyframes(
        &mut *self.packet_orchestrator.borrow_mut(),
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
      if let Some(previous_area) = self.areas.get_mut(&bot.area_id) {
        previous_area.remove_bot(id);

        broadcast_to_area(
          &mut *self.packet_orchestrator.borrow_mut(),
          previous_area,
          Reliability::ReliableOrdered,
          ServerPacket::ActorDisconnected {
            ticket: id,
            warp_out: warp_in,
          },
        );
      }

      bot.area_id = area_id.to_string();
      bot.x = x;
      bot.y = y;
      bot.z = z;

      let area = self.areas.get_mut(area_id).unwrap();
      area.add_bot(id.to_string());

      ensure_assets(
        &mut *self.packet_orchestrator.borrow_mut(),
        self.config.max_payload_size,
        &self.asset_manager,
        &mut self.clients,
        area.get_connected_players(),
        [bot.texture_path.as_str(), bot.animation_path.as_str()].iter(),
      );

      broadcast_to_area(
        &mut *self.packet_orchestrator.borrow_mut(),
        area,
        Reliability::ReliableOrdered,
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
  pub(super) fn set_active_plugin(&mut self, active_plugin: usize) {
    self.active_plugin = active_plugin;
  }

  pub(super) fn tick(&mut self) {
    self.broadcast_bot_positions();
    self.broadcast_map_changes();
  }

  fn broadcast_bot_positions(&mut self) {
    use std::time::Instant;

    let now = Instant::now();

    for bot in self.bots.values() {
      let time_since_last_movement = now - bot.last_movement_time;

      if time_since_last_movement.as_secs_f32() > self.config.max_idle_packet_duration {
        continue;
      }

      let area = match self.areas.get(&bot.area_id) {
        Some(area) => area,
        None => continue,
      };

      let packet = ServerPacket::ActorMove {
        ticket: &bot.id,
        x: bot.x,
        y: bot.y,
        z: bot.z,
        direction: bot.direction,
      };

      broadcast_to_area(
        &mut *self.packet_orchestrator.borrow_mut(),
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

        self.asset_manager.set_asset(map_path.clone(), map_asset);
        update_cached_clients(
          &mut *self.packet_orchestrator.borrow_mut(),
          self.config.max_payload_size,
          &self.asset_manager,
          &mut self.clients,
          &map_path,
        );

        let packet = ServerPacket::MapUpdate {
          map_path: &map_path,
        };

        broadcast_to_area(
          &mut *self.packet_orchestrator.borrow_mut(),
          area,
          Reliability::ReliableOrdered,
          packet,
        );
      }
    }
  }
}

fn broadcast_actor_keyframes(
  packet_orchestrator: &mut PacketOrchestrator,
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

  let mut packets = Vec::new();
  use crate::packets::build_packet;

  for chunk in chunks {
    if let Some(chunk) = last_chunk {
      packets.push(build_packet(ServerPacket::ActorPropertyKeyFrames {
        ticket: id,
        tail: false,
        keyframes: chunk,
      }));
    }

    last_chunk = Some(chunk)
  }

  if let Some(chunk) = last_chunk {
    packets.push(build_packet(ServerPacket::ActorPropertyKeyFrames {
      ticket: id,
      tail: true,
      keyframes: chunk,
    }));
  }

  packet_orchestrator.broadcast_byte_packets_to_room(
    area.get_id(),
    Reliability::ReliableOrdered,
    &packets,
  );
}

fn update_cached_clients(
  packet_orchestrator: &mut PacketOrchestrator,
  max_payload_size: usize,
  asset_manager: &AssetManager,
  clients: &mut HashMap<String, Client>,
  asset_path: &str,
) {
  use crate::packets::build_packet;

  let mut dependencies = asset_manager.get_flattened_dependency_chain(asset_path);
  dependencies.pop();

  let reliability = Reliability::ReliableOrdered;

  let mut clients_to_update: Vec<&mut Client> = clients
    .values_mut()
    .filter(|client| client.cached_assets.contains(asset_path))
    .collect();

  // ensuring dependencies
  for asset_path in dependencies {
    if let Some(asset) = asset_manager.get_asset(asset_path) {
      let mut byte_vecs = Vec::new();

      for client in &mut clients_to_update {
        if client.cached_assets.contains(asset_path) {
          continue;
        }

        client.cached_assets.insert(asset_path.to_string());

        // lazily create stream
        if byte_vecs.is_empty() {
          byte_vecs = create_asset_stream(max_payload_size, asset_path, asset)
            .into_iter()
            .map(build_packet)
            .collect();
        }

        packet_orchestrator.send_byte_packets(client.socket_address, reliability, &byte_vecs);
      }
    }
  }

  // updating clients who have this asset
  if let Some(asset) = asset_manager.get_asset(asset_path) {
    let byte_vecs: Vec<Vec<u8>> = create_asset_stream(max_payload_size, asset_path, asset)
      .into_iter()
      .map(build_packet)
      .collect();

    for client in &mut clients_to_update {
      packet_orchestrator.send_byte_packets(client.socket_address, reliability, &byte_vecs);
    }
  }
}

fn broadcast_to_area(
  packet_orchestrator: &mut PacketOrchestrator,
  area: &Area,
  reliability: Reliability,
  packet: ServerPacket,
) {
  packet_orchestrator.broadcast_to_room(area.get_id(), reliability, packet);
}

fn ensure_asset(
  packet_orchestrator: &mut PacketOrchestrator,
  max_payload_size: usize,
  asset_manager: &AssetManager,
  clients: &mut HashMap<String, Client>,
  player_ids: &[String],
  asset_path: &str,
) {
  if !asset_path.starts_with("/server") {
    return;
  }

  let assets_to_send = asset_manager.get_flattened_dependency_chain(asset_path);

  if assets_to_send.is_empty() {
    warn!("No asset found with path \"{}\"", asset_path);
    return;
  }

  for asset_path in assets_to_send {
    let asset = if let Some(asset) = asset_manager.get_asset(asset_path) {
      asset
    } else {
      warn!("No asset found with path \"{}\"", asset_path);
      continue;
    };

    let mut byte_vecs = Vec::new();

    for player_id in player_ids {
      let client = clients.get_mut(player_id).unwrap();

      if asset.cachable && client.cached_assets.contains(asset_path) {
        continue;
      }

      // lazily create stream
      if byte_vecs.is_empty() {
        use crate::packets::build_packet;

        byte_vecs = create_asset_stream(max_payload_size, asset_path, asset)
          .into_iter()
          .map(build_packet)
          .collect();
      }

      if asset.cachable {
        client.cached_assets.insert(asset_path.to_string());
      }

      packet_orchestrator.send_byte_packets(
        client.socket_address,
        Reliability::ReliableOrdered,
        &byte_vecs,
      );
    }
  }
}

fn ensure_assets<'a, I>(
  packet_orchestrator: &mut PacketOrchestrator,
  max_payload_size: usize,
  asset_manager: &AssetManager,
  clients: &mut HashMap<String, Client>,
  player_ids: &[String],
  asset_paths: I,
) where
  I: std::iter::Iterator<Item = &'a &'a str>,
{
  for asset_path in asset_paths {
    ensure_asset(
      packet_orchestrator,
      max_payload_size,
      asset_manager,
      clients,
      player_ids,
      asset_path,
    );
  }
}
