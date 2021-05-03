use super::boot::Boot;
use super::client::Client;
use super::map::Map;
use super::server::ServerConfig;
use super::{Actor, Area, Asset, AssetData, BbsPost, Direction};
use crate::packets::{create_asset_stream, Reliability, ServerPacket};
use crate::threads::worker_threads::{Job, JobGiver};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Net {
  socket: Rc<UdpSocket>,
  max_payload_size: usize,
  resend_budget: usize,
  avatar_dimensions_limit: u32,
  areas: HashMap<String, Area>,
  clients: HashMap<String, Client>,
  bots: HashMap<String, Actor>,
  assets: HashMap<String, Asset>,
  active_script: usize,
  kick_list: Vec<Boot>,
  job_giver: JobGiver,
}

impl Net {
  pub fn new(socket: Rc<UdpSocket>, server_config: &ServerConfig) -> Net {
    use super::asset::get_map_path;
    use crate::threads::create_worker_threads;
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
          let mut map = Map::from(&raw_map);

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
      avatar_dimensions_limit: server_config.avatar_dimensions_limit,
      areas,
      clients: HashMap::new(),
      bots: HashMap::new(),
      assets,
      active_script: 0,
      kick_list: Vec::new(),
      job_giver: create_worker_threads(server_config.worker_thread_count),
    }
  }

  fn load_assets_from_dir(assets: &mut HashMap<String, Asset>, dir: &std::path::Path) {
    use super::load_asset;
    use std::fs::read_dir;

    if let Ok(entries) = read_dir(dir) {
      for wrapped_entry in entries {
        if let Ok(entry) = wrapped_entry {
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
  }

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

  #[allow(dead_code)]
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
        self.max_payload_size,
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
        self.max_payload_size,
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

  pub fn set_player_emote(&mut self, id: &str, emote_id: u8) {
    if let Some(client) = self.clients.get(id) {
      let packet = ServerPacket::ActorEmote {
        ticket: id.to_string(),
        emote_id,
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

  pub fn exclusive_player_emote(&mut self, target_id: &str, emoter_id: &str, emote_id: u8) {
    if let Some(client) = self.clients.get_mut(target_id) {
      let packet = ServerPacket::ActorEmote {
        ticket: emoter_id.to_string(),
        emote_id,
      };

      client
        .packet_shipper
        .send(&self.socket, &Reliability::Reliable, &packet);
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

  pub fn is_player_in_widget(&self, id: &str) -> bool {
    if let Some(client) = self.clients.get(id) {
      return client.is_in_widget();
    }

    false
  }

  pub fn preload_asset_for_player(&mut self, id: &str, asset_path: &str) {
    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[String::from(id)],
      asset_path,
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
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
        &Reliability::ReliableOrdered,
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
        &Reliability::ReliableOrdered,
        &ServerPacket::ExcludeObject { id: object_id },
      );
    }
  }

  pub fn include_object_for_player(&mut self, id: &str, object_id: u32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::IncludeObject { id: object_id },
      );
    }
  }

  pub fn move_player_camera(&mut self, id: &str, x: f32, y: f32, z: f32, hold_time: f32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::MoveCamera { x, y, z, hold_time },
      );
    }
  }

  pub fn slide_player_camera(&mut self, id: &str, x: f32, y: f32, z: f32, duration: f32) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::SlideCamera { x, y, z, duration },
      );
    }
  }

  pub fn unlock_player_camera(&mut self, id: &str) {
    if let Some(client) = self.clients.get_mut(id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::UnlockCamera,
      );
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
        &Reliability::ReliableOrdered,
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
    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      &mug_texture_path,
    );

    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      &mug_animation_path,
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_script);

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
    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      &mug_texture_path,
    );

    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      &mug_animation_path,
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_script);

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
    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      &mug_texture_path,
    );

    assert_asset(
      &self.socket,
      self.max_payload_size,
      &self.assets,
      &mut self.clients,
      &[id.to_string()],
      &mug_animation_path,
    );

    if let Some(client) = self.clients.get_mut(id) {
      client.widget_tracker.track_textbox(self.active_script);

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

  pub fn open_board(
    &mut self,
    player_id: &str,
    name: String,
    color: (u8, u8, u8),
    posts: Vec<BbsPost>,
  ) {
    use super::bbs_post::count_fit_posts;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    // reliability + id + type
    let header_size = 1 + 8 + 2;

    let mut packet_size = header_size;
    packet_size += name.len() + 1;
    packet_size += 3; // color

    let fit_post_count = count_fit_posts(self.max_payload_size - packet_size, 0, &posts);

    client.packet_shipper.send(
      &self.socket,
      &Reliability::ReliableOrdered,
      &ServerPacket::OpenBoard {
        current_depth: client.widget_tracker.get_board_count() as u8,
        name,
        color,
        posts: &posts[..fit_post_count],
      },
    );

    client.widget_tracker.track_board(self.active_script);

    let mut last_id = &posts[fit_post_count - 1].id;
    let mut start_index = 0;
    start_index += fit_post_count;

    while start_index < posts.len() {
      let mut packet_size = header_size;
      packet_size += 2; // currentDepth + hasReference
      packet_size += last_id.len() + 1; // reference

      let fit_post_count =
        count_fit_posts(self.max_payload_size - packet_size, start_index, &posts);

      if fit_post_count == 0 {
        println!("open_board failed! (Contains a post too large to send)");
        break;
      }

      let end_index = start_index + fit_post_count - 1;

      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::AppendPosts {
          current_depth: client.widget_tracker.get_board_count() as u8,
          reference: Some(last_id.clone()),
          posts: &posts[start_index..end_index + 1],
        },
      );

      last_id = &posts[end_index].id;
      start_index = end_index + 1;
    }
  }

  pub fn prepend_posts(&mut self, player_id: &str, reference: Option<String>, posts: Vec<BbsPost>) {
    use super::bbs_post::count_fit_posts;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    // reliability + id + type
    let header_size = 1 + 8 + 2;

    let mut packet_size = header_size;
    packet_size += 2; // currentDepth + hasReference

    if let Some(reference) = reference.as_ref() {
      packet_size += reference.len() + 1; // reference
    }

    let fit_post_count = count_fit_posts(self.max_payload_size - packet_size, 0, &posts);

    client.packet_shipper.send(
      &self.socket,
      &Reliability::ReliableOrdered,
      &ServerPacket::PrependPosts {
        current_depth: client.widget_tracker.get_board_count() as u8,
        reference,
        posts: &posts[..fit_post_count],
      },
    );

    let mut last_id = &posts[fit_post_count - 1].id;
    let mut start_index = 0;
    start_index += fit_post_count;

    while start_index < posts.len() {
      let mut packet_size = header_size;
      packet_size += 2; // currentDepth + hasReference
      packet_size += last_id.len() + 1; // reference

      let fit_post_count =
        count_fit_posts(self.max_payload_size - packet_size, start_index, &posts);

      if fit_post_count == 0 {
        println!("prepend_posts failed! (Contains a post too large to send)");
        break;
      }

      let end_index = start_index + fit_post_count - 1;

      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::AppendPosts {
          current_depth: client.widget_tracker.get_board_count() as u8,
          reference: Some(last_id.clone()),
          posts: &posts[start_index..end_index + 1],
        },
      );

      last_id = &posts[end_index].id;
      start_index = end_index + 1;
    }
  }

  pub fn append_posts(&mut self, player_id: &str, reference: Option<String>, posts: Vec<BbsPost>) {
    use super::bbs_post::count_fit_posts;

    let client = if let Some(client) = self.clients.get_mut(player_id) {
      client
    } else {
      return;
    };

    // reliability + id + type
    let header_size = 1 + 8 + 2;

    let mut last_id = reference;
    let mut start_index = 0;

    while start_index < posts.len() {
      let mut packet_size = header_size;
      packet_size += 2; // currentDepth + hasReference

      if let Some(last_id) = last_id.as_ref() {
        packet_size += last_id.len() + 1; // reference
      }

      let fit_post_count =
        count_fit_posts(self.max_payload_size - packet_size, start_index, &posts);

      if fit_post_count == 0 {
        println!("append_posts failed! (Contains a post too large to send)");
        break;
      }

      let end_index = start_index + fit_post_count - 1;

      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::AppendPosts {
          current_depth: client.widget_tracker.get_board_count() as u8,
          reference: last_id,
          posts: &posts[start_index..end_index + 1],
        },
      );

      last_id = Some(posts[end_index].id.clone());
      start_index = end_index + 1;
    }
  }

  pub fn remove_post(&mut self, player_id: &str, post_id: &str) {
    if let Some(client) = self.clients.get_mut(player_id) {
      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::RemovePost {
          current_depth: client.widget_tracker.get_board_count() as u8,
          id: post_id.to_string(),
        },
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

    let texture_path;
    let animation_path;

    if let Some(client) = self.clients.get_mut(id) {
      let previous_area = self.areas.get_mut(&client.actor.area_id).unwrap();

      if !previous_area
        .get_connected_players()
        .contains(&id.to_string())
      {
        // client has not been added to any area yet
        // assume client was transferred on initial connection by a plugin
        client.actor.area_id = area_id.to_string();
        client.warp_in = warp_in;
        client.warp_x = x;
        client.warp_y = y;
        client.warp_z = z;
        client.warp_direction = direction;
        return;
      }

      texture_path = client.actor.texture_path.clone();
      animation_path = client.actor.animation_path.clone();

      client.packet_shipper.send(
        &self.socket,
        &Reliability::ReliableOrdered,
        &ServerPacket::TransferStart { warp_out: warp_in },
      );

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
    } else {
      // allows us to safely unwrap after this
      // as long as send_area doesn't delete the client (why would it?)
      return;
    }

    let area = self.areas.get_mut(area_id).unwrap();

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

    area.add_player(id.to_string());
    self.send_area(id, &area_id);

    let mut client = self.clients.get_mut(id).unwrap();

    client.actor.area_id = area_id.to_string();
    client.warp_in = warp_in;
    client.warp_x = x;
    client.warp_y = y;
    client.warp_z = z;
    client.warp_direction = direction;
    client.transferring = true;
    client.ready = false;

    client.packet_shipper.send(
      &self.socket,
      &Reliability::ReliableOrdered,
      &ServerPacket::Teleport {
        warp: false,
        x,
        y,
        z,
        direction,
      },
    );

    client.packet_shipper.send(
      &self.socket,
      &Reliability::ReliableOrdered,
      &ServerPacket::TransferComplete { warp_in, direction },
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
        &Reliability::ReliableOrdered,
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
      self.resend_budget,
    );

    let id = client.actor.id.clone();

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

    if find_longest_frame_length(&animation_data) > self.avatar_dimensions_limit {
      let reason = format!(
        "Avatar has frames larger than limit {}x{}",
        self.avatar_dimensions_limit, self.avatar_dimensions_limit
      );

      self.kick_player(player_id, &reason, true);

      return None;
    }

    let texture_path = asset::get_player_texture_path(player_id);
    let animation_path = asset::get_player_animation_path(player_id);
    let mugshot_texture_path = asset::get_player_mugshot_texture_path(player_id);
    let mugshot_animation_path = asset::get_player_mugshot_animation_path(player_id);

    self.set_asset(
      texture_path.clone(),
      Asset {
        data: AssetData::Texture(texture_data),
        dependencies: Vec::new(),
        last_modified: 0,
        cachable: false,
      },
    );

    self.set_asset(
      animation_path.clone(),
      Asset {
        data: AssetData::Text(animation_data),
        dependencies: Vec::new(),
        last_modified: 0,
        cachable: false,
      },
    );

    self.set_asset(
      mugshot_texture_path,
      Asset {
        data: AssetData::Texture(mugshot_texture_data),
        dependencies: Vec::new(),
        last_modified: 0,
        cachable: false,
      },
    );

    self.set_asset(
      mugshot_animation_path,
      Asset {
        data: AssetData::Text(mugshot_animation_data),
        dependencies: Vec::new(),
        last_modified: 0,
        cachable: false,
      },
    );

    Some((texture_path, animation_path))
  }

  pub(super) fn connect_client(&mut self, player_id: &str) {
    let client = self.clients.get(player_id).unwrap();
    let area_id = client.actor.area_id.clone();
    let texture_path = client.actor.texture_path.clone();
    let animation_path = client.actor.animation_path.clone();

    let area = self.areas.get_mut(&area_id).unwrap();
    area.add_player(client.actor.id.clone());

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
      .send(&self.socket, &Reliability::ReliableOrdered, &packet);
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

      asset_paths.push(other_client.actor.texture_path.clone());
      asset_paths.push(other_client.actor.animation_path.clone());

      packets.push(ServerPacket::ActorConnected {
        ticket: other_client.actor.id.clone(),
        name: other_client.actor.name.clone(),
        texture_path: other_client.actor.texture_path.clone(),
        animation_path: other_client.actor.animation_path.clone(),
        direction: other_client.actor.direction,
        x: other_client.actor.x,
        y: other_client.actor.y,
        z: other_client.actor.z,
        solid: other_client.actor.solid,
        warp_in: false,
      });
    }

    // send bots
    for bot_id in area.get_connected_bots() {
      let bot = self.bots.get(bot_id).unwrap();

      asset_paths.push(bot.texture_path.clone());
      asset_paths.push(bot.animation_path.clone());

      packets.push(ServerPacket::ActorConnected {
        ticket: bot.id.clone(),
        name: bot.name.clone(),
        texture_path: bot.texture_path.clone(),
        animation_path: bot.animation_path.clone(),
        direction: bot.direction,
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
      client.transferring = false;

      // clone id to end mutable client lifetime
      let player_id = client.actor.id.clone();
      let area = self.areas.get_mut(&client.actor.area_id).unwrap();
      let texture_path = client.actor.texture_path.clone();
      let animation_path = client.actor.animation_path.clone();

      let packet = ServerPacket::ActorConnected {
        ticket: player_id,
        name: client.actor.name.clone(),
        texture_path,
        animation_path,
        direction: client.actor.direction,
        x: client.warp_x,
        y: client.warp_y,
        z: client.warp_z,
        solid: client.actor.solid,
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

  pub(super) fn remove_player(&mut self, id: &str, warp_out: bool) {
    use super::asset;
    if let Some(client) = self.clients.remove(id) {
      self.assets.remove(&asset::get_player_texture_path(id));
      self.assets.remove(&asset::get_player_animation_path(id));

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
    if let Some(area) = self.areas.get_mut(&bot.area_id) {
      area.add_bot(bot.id.clone());

      let packet = ServerPacket::ActorConnected {
        ticket: bot.id.clone(),
        name: bot.name.clone(),
        texture_path: bot.texture_path.clone(),
        animation_path: bot.animation_path.clone(),
        direction: bot.direction,
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

  pub fn set_bot_emote(&mut self, id: &str, emote_id: u8) {
    if let Some(bot) = self.bots.get(id) {
      let packet = ServerPacket::ActorEmote {
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
        ServerPacket::ActorConnected {
          ticket: id.to_string(),
          name: bot.name.clone(),
          texture_path: bot.texture_path.clone(),
          animation_path: bot.animation_path.clone(),
          direction: bot.direction,
          x: bot.x,
          y: bot.y,
          z: bot.z,
          solid: bot.solid,
          warp_in,
        },
      );
    }
  }

  pub fn message_server(&mut self, address: String, port: u16, data: Vec<u8>) {
    use crate::jobs::message_server::message_server;

    if let Ok(socket) = self.socket.try_clone() {
      let job = message_server(socket, address, port, data);
      self.add_job(job);
    }
  }

  pub fn add_job(&mut self, job: Job) {
    self.job_giver.give_job(job);
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
