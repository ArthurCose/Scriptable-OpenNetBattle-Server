use super::boot::Boot;
use super::plugin_wrapper::PluginWrapper;
use super::Net;
use crate::packets::{
  build_unreliable_packet, ClientPacket, PacketOrchestrator, PacketSorter, Reliability,
  ServerPacket,
};
use crate::plugins::PluginInterface;
use crate::threads::{create_clock_thread, create_listening_thread, ThreadMessage};
use log::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

#[derive(Clone)]
pub struct ServerConfig {
  pub public_ip: std::net::IpAddr,
  pub port: u16,
  pub log_connections: bool,
  pub log_packets: bool,
  pub max_payload_size: usize,
  pub resend_budget: usize,
  pub receiving_drop_rate: f32,
  pub player_asset_limit: usize,
  pub avatar_dimensions_limit: u32,
  pub custom_emotes_path: Option<String>,
  pub max_idle_packet_duration: f32,
  pub max_silence_duration: f32,
  pub heartbeat_rate: f32,
}

pub struct Server {
  player_id_map: HashMap<std::net::SocketAddr, String>,
  packet_sorter_map: HashMap<std::net::SocketAddr, PacketSorter>,
  plugin_wrapper: PluginWrapper,
  config: Rc<ServerConfig>,
}

impl Server {
  pub fn new(config: ServerConfig) -> Server {
    Server {
      player_id_map: HashMap::new(),
      packet_sorter_map: HashMap::new(),
      plugin_wrapper: PluginWrapper::new(),
      config: Rc::new(config),
    }
  }

  pub fn add_plugin_interface(&mut self, plugin_interface: Box<dyn PluginInterface>) {
    self.plugin_wrapper.add_plugin_interface(plugin_interface);
  }

  pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::mpsc;
    use std::time::Instant;

    let addr = format!("0.0.0.0:{}", self.config.port);
    let socket = UdpSocket::bind(addr)?;

    socket.take_error()?;

    info!("Server listening on: {}", self.config.port);

    let socket = Rc::new(socket);
    let packet_orchestrator = Rc::new(RefCell::new(PacketOrchestrator::new(
      socket.clone(),
      self.config.resend_budget,
    )));

    let mut net = Net::new(
      socket.clone(),
      packet_orchestrator.clone(),
      self.config.clone(),
    );

    self.plugin_wrapper.init(&mut net);

    let (tx, rx) = mpsc::channel();
    create_clock_thread(tx.clone());
    create_listening_thread(tx, socket.try_clone()?, (*self.config).clone());

    info!("Server started");

    let mut time = Instant::now();
    let mut last_heartbeat = Instant::now();

    loop {
      match rx.recv()? {
        ThreadMessage::Tick(started) => {
          started();

          let elapsed_time = time.elapsed();
          time = Instant::now();

          self
            .plugin_wrapper
            .tick(&mut net, elapsed_time.as_secs_f32());

          // kick silent clients
          let mut kick_list = Vec::new();

          for (socket_address, packet_sorter) in &mut self.packet_sorter_map {
            let last_message = packet_sorter.get_last_message_time();

            if last_message.elapsed().as_secs_f32() > self.config.max_silence_duration {
              kick_list.push(Boot {
                socket_address: *socket_address,
                reason: String::from("packet silence"),
                warp_out: true,
              });
            }
          }

          kick_list.extend(net.take_kick_list());

          // actually kick clients
          for boot in kick_list {
            self.disconnect_client(&mut net, &boot.socket_address, &boot.reason, boot.warp_out);

            // send reason
            let buf = build_unreliable_packet(ServerPacket::Kick {
              reason: &boot.reason,
            });

            let _ = socket.send_to(&buf, boot.socket_address);
          }

          packet_orchestrator.borrow_mut().resend_backed_up_packets();

          net.tick();

          if last_heartbeat.elapsed().as_secs_f32() >= self.config.heartbeat_rate {
            packet_orchestrator
              .borrow_mut()
              .broadcast(Reliability::Reliable, ServerPacket::Heartbeat);

            last_heartbeat = time;
          }
        }
        ThreadMessage::ClientPacket {
          socket_address,
          headers,
          packet,
        } => {
          let is_reliable = headers.reliability.is_reliable();

          if headers.id == 0 && is_reliable && !self.packet_sorter_map.contains_key(&socket_address)
          {
            // received the first reliable packet, store a new connection
            let packet_sorter = PacketSorter::new(socket_address);
            self.packet_sorter_map.insert(socket_address, packet_sorter);

            if self.config.log_connections {
              debug!("{} connected", socket_address);
            }
          }

          if let Some(packet_sorter) = self.packet_sorter_map.get_mut(&socket_address) {
            let packets = packet_sorter.sort_packet(&socket, headers, packet);

            for packet in packets {
              self.handle_packet(
                &mut net,
                &packet_orchestrator,
                &socket,
                socket_address,
                packet,
              );
            }
          } else if !is_reliable {
            // ignoring errors, no packet sorter = never connected
            let _ = self.handle_packet(
              &mut net,
              &packet_orchestrator,
              &socket,
              socket_address,
              packet,
            );
          }
        }
      }
    }
  }

  fn handle_packet(
    &mut self,
    net: &mut Net,
    packet_orchestrator: &RefCell<PacketOrchestrator>,
    socket: &std::net::UdpSocket,
    socket_address: std::net::SocketAddr,
    client_packet: ClientPacket,
  ) {
    if let Some(player_id) = self.player_id_map.get(&socket_address) {
      match client_packet {
        ClientPacket::VersionRequest => {
          if self.config.log_packets {
            debug!("Received bad VersionRequest packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(ServerPacket::VersionInfo {
            max_payload_size: self.config.max_payload_size,
          });
          let _ = socket.send_to(&buf, socket_address);
        }
        ClientPacket::Heartbeat => {
          if self.config.log_packets {
            debug!("Received Heartbeat packet from {}", socket_address);
          }
        }
        ClientPacket::AssetFound {
          path,
          last_modified,
        } => {
          if self.config.log_packets {
            debug!("Received AssetFound packet from {}", socket_address);
          }

          let mut is_valid = false;

          if let Some(asset) = net.get_asset(&path) {
            is_valid = asset.last_modified == last_modified;
          }

          if is_valid {
            if let Some(client) = net.get_client_mut(player_id) {
              client.cached_assets.insert(path);
            }
          } else {
            packet_orchestrator.borrow_mut().send(
              socket_address,
              Reliability::ReliableOrdered,
              ServerPacket::RemoveAsset { path: &path },
            );
          }
        }
        ClientPacket::AssetStream { asset_type, data } => {
          if self.config.log_packets {
            debug!("Received AssetStream packet from {}", socket_address);
          }

          let client = net.get_client_mut(player_id).unwrap();

          let asset_buffer = match asset_type {
            0 => Some(&mut client.texture_buffer),
            1 => Some(&mut client.animation_buffer),
            2 => Some(&mut client.mugshot_texture_buffer),
            3 => Some(&mut client.mugshot_animation_buffer),
            _ => None,
          };

          if let Some(asset_buffer) = asset_buffer {
            if asset_buffer.len() < self.config.player_asset_limit {
              asset_buffer.extend(data);
            } else {
              let reason = format!(
                "Avatar asset larger than {}KiB",
                self.config.player_asset_limit / 1024
              );

              net.kick_player(player_id, &reason, true);
            }
          }
        }
        ClientPacket::Ack { reliability, id } => {
          if self.config.log_packets {
            debug!(
              "Received Ack for {:?} {} from {}",
              reliability, id, socket_address
            );
          }

          packet_orchestrator
            .borrow_mut()
            .acknowledged(socket_address, reliability, id);
        }
        ClientPacket::Authorize {
          origin_address: _,
          port: _,
          identity: _,
          data: _,
        } => {
          if self.config.log_packets {
            debug!("Received bad Authorize packet from {}", socket_address);
          }
        }
        ClientPacket::Login {
          username: _,
          identity: _,
          data: _,
        } => {
          if self.config.log_packets {
            debug!("Received bad Login packet from {}", socket_address);
          }
        }
        ClientPacket::RequestJoin => {
          if self.config.log_packets {
            debug!("Received RequestJoin packet from {}", socket_address);
          }

          net.spawn_client(player_id);

          self.plugin_wrapper.handle_player_connect(net, player_id);

          net.connect_client(player_id);

          if self.config.log_connections {
            debug!("{} connected", player_id);
          }
        }
        ClientPacket::Logout => {
          if self.config.log_packets {
            debug!("Received Logout packet from {}", socket_address);
          }

          self.disconnect_client(net, &socket_address, "Leaving", true);
        }
        ClientPacket::Position {
          creation_time,
          x,
          y,
          z,
          direction,
        } => {
          if self.config.log_packets {
            debug!("Received Position packet from {}", socket_address);
          }

          let client = net.get_client_mut(player_id).unwrap();

          if client.ready && creation_time > client.area_join_time {
            #[allow(clippy::float_cmp)]
            let position_changed =
              client.actor.x != x || client.actor.y != y || client.actor.z != z;

            if position_changed {
              client.actor.current_animation = None;

              self
                .plugin_wrapper
                .handle_player_move(net, player_id, x, y, z);
            }

            net.update_player_position(player_id, x, y, z, direction);
          }
        }
        ClientPacket::Ready { time } => {
          if self.config.log_packets {
            debug!("Received Ready packet from {}", socket_address);
          }

          let client = net.get_client_mut(player_id).unwrap();

          client.area_join_time = time;
          client.actor.x = client.warp_x;
          client.actor.y = client.warp_y;
          client.actor.z = client.warp_z;

          if client.transferring {
            self.plugin_wrapper.handle_player_transfer(net, player_id);
          } else {
            self.plugin_wrapper.handle_player_join(net, player_id);
          }

          net.mark_client_ready(player_id);
        }
        ClientPacket::TransferredOut => {
          if self.config.log_packets {
            debug!("Received TransferredOut packet from {}", socket_address);
          }

          net.complete_transfer(player_id);
        }
        ClientPacket::CustomWarp { tile_object_id } => {
          if self.config.log_packets {
            debug!("Received CustomWarp packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_custom_warp(net, player_id, tile_object_id);
        }
        ClientPacket::AvatarChange {
          name,
          element,
          max_health,
        } => {
          if self.config.log_packets {
            debug!("Received AvatarChange packet from {}", socket_address);
          }

          if let Some((texture_path, animation_path)) = net.store_player_assets(player_id) {
            net.update_player_data(player_id, element.clone(), max_health);

            let prevent_default = self.plugin_wrapper.handle_player_avatar_change(
              net,
              player_id,
              &texture_path,
              &animation_path,
              &name,
              &element,
              max_health,
            );

            if !prevent_default {
              net.set_player_avatar(player_id, &texture_path, &animation_path);
            }
          }
        }
        ClientPacket::Emote { emote_id } => {
          if self.config.log_packets {
            debug!("Received Emote packet from {}", socket_address);
          }

          let prevent_default = self
            .plugin_wrapper
            .handle_player_emote(net, player_id, emote_id);

          if !prevent_default {
            net.set_player_emote(player_id, emote_id, false);
          }
        }
        ClientPacket::ObjectInteraction {
          tile_object_id,
          button,
        } => {
          if self.config.log_packets {
            debug!("Received ObjectInteraction packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_object_interaction(net, player_id, tile_object_id, button);
        }
        ClientPacket::ActorInteraction { actor_id, button } => {
          if self.config.log_packets {
            debug!("Received ActorInteraction packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_actor_interaction(net, player_id, &actor_id, button);
        }
        ClientPacket::TileInteraction { x, y, z, button } => {
          if self.config.log_packets {
            debug!("Received TileInteraction packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_tile_interaction(net, player_id, x, y, z, button);
        }
        ClientPacket::TextBoxResponse { response } => {
          if self.config.log_packets {
            debug!("Received TextBoxResponse packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_textbox_response(net, player_id, response);
        }
        ClientPacket::PromptResponse { response } => {
          if self.config.log_packets {
            debug!("Received PromptResponse packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_prompt_response(net, player_id, response);
        }
        ClientPacket::BoardOpen => {
          if self.config.log_packets {
            debug!("Received BoardOpen packet from {}", socket_address);
          }

          self.plugin_wrapper.handle_board_open(net, player_id);
        }
        ClientPacket::BoardClose => {
          if self.config.log_packets {
            debug!("Received BoardClose packet from {}", socket_address);
          }

          self.plugin_wrapper.handle_board_close(net, player_id);
        }
        ClientPacket::PostRequest => {
          if self.config.log_packets {
            debug!("Received PostRequest packet from {}", socket_address);
          }

          self.plugin_wrapper.handle_post_request(net, player_id);
        }
        ClientPacket::PostSelection { post_id } => {
          if self.config.log_packets {
            debug!("Received PostSelection packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_post_selection(net, player_id, &post_id);

          packet_orchestrator.borrow_mut().send(
            socket_address,
            Reliability::ReliableOrdered,
            ServerPacket::PostSelectionAck,
          );
        }
        ClientPacket::ShopClose => {
          if self.config.log_packets {
            debug!("Received ShopClose packet from {}", socket_address);
          }

          self.plugin_wrapper.handle_shop_close(net, player_id);
        }
        ClientPacket::ShopPurchase { item_name } => {
          if self.config.log_packets {
            debug!("Received ShopPurchase packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_shop_purchase(net, player_id, &item_name);
        }
        ClientPacket::BattleResults { battle_stats } => {
          if self.config.log_packets {
            debug!("Received BattleResults packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_battle_results(net, player_id, &battle_stats);
        }
        ClientPacket::ServerMessage { data } => {
          // this should never happen but 🤷‍♂️
          if self.config.log_packets {
            debug!("Received ServerMessage packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_server_message(net, socket_address, &data);
        }
      }
    } else {
      match client_packet {
        ClientPacket::VersionRequest => {
          if self.config.log_packets {
            debug!("Received VersionRequest packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(ServerPacket::VersionInfo {
            max_payload_size: self.config.max_payload_size,
          });
          let _ = socket.send_to(&buf, socket_address);
        }
        ClientPacket::Authorize {
          origin_address,
          port,
          identity,
          data,
        } => {
          self
            .plugin_wrapper
            .handle_authorization(net, &identity, &origin_address, port, &data);
        }
        ClientPacket::Login {
          username,
          identity,
          data,
        } => {
          if self.config.log_packets {
            debug!("Received Login packet from {}", socket_address);
          }

          let player_id = net.add_client(socket_address, username, identity);

          self.player_id_map.insert(socket_address, player_id.clone());

          self
            .plugin_wrapper
            .handle_player_request(net, &player_id, &data);
        }
        ClientPacket::ServerMessage { data } => {
          self
            .plugin_wrapper
            .handle_server_message(net, socket_address, &data);
        }
        _ => {
          if self.config.log_packets {
            debug!("Received bad packet from {}", socket_address);
            debug!("{:?}", client_packet);
            debug!("Connected clients: {:?}", self.player_id_map.keys());
          }
        }
      }
    }
  }

  fn disconnect_client(
    &mut self,
    net: &mut Net,
    socket_address: &std::net::SocketAddr,
    reason: &str,
    warp_out: bool,
  ) {
    if let Some(player_id) = self.player_id_map.remove(socket_address) {
      self
        .plugin_wrapper
        .handle_player_disconnect(net, &player_id);

      net.remove_player(&player_id, warp_out);

      if self.config.log_connections {
        debug!("{} disconnected for {}", player_id, reason);
      }
    }

    self.packet_sorter_map.remove(socket_address);

    if self.config.log_connections {
      debug!("{} disconnected for {}", socket_address, reason);
    }
  }
}
