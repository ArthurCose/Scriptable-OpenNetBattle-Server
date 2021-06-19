use super::boot::Boot;
use super::plugin_wrapper::PluginWrapper;
use super::Net;
use crate::packets::{
  build_unreliable_packet, ClientPacket, PacketSorter, Reliability, ServerPacket,
};
use crate::plugins::PluginInterface;
use crate::threads::{create_clock_thread, create_listening_thread, ThreadMessage};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct ServerConfig {
  pub public_ip: std::net::IpAddr,
  pub port: u16,
  pub log_connections: bool,
  pub log_packets: bool,
  pub max_payload_size: usize,
  pub resend_budget: usize,
  pub player_asset_limit: usize,
  pub avatar_dimensions_limit: u32,
  pub worker_thread_count: u16,
  pub custom_emotes_path: Option<String>,
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

    println!("Server listening on: {}", self.config.port);

    let socket = Rc::new(socket);
    let mut net = Net::new(socket.clone(), self.config.clone());

    self.plugin_wrapper.init(&mut net);

    let (tx, rx) = mpsc::channel();
    create_clock_thread(tx.clone());
    create_listening_thread(
      tx,
      socket.try_clone()?,
      self.config.max_payload_size,
      self.config.log_packets,
    );

    println!("Server started");

    let mut time = Instant::now();

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
          let max_silence = std::time::Duration::from_secs(5);

          for (socket_address, packet_sorter) in &mut self.packet_sorter_map {
            let last_message = packet_sorter.get_last_message_time();

            if last_message.elapsed() > max_silence {
              kick_list.push(Boot {
                socket_address: *socket_address,
                reason: String::from("packet silence"),
                warp_out: true,
              });
            }
          }

          kick_list.extend(net.get_kick_list().iter().cloned());
          net.clear_kick_list();

          // actually kick clients
          for boot in kick_list {
            self.disconnect_client(&mut net, &boot.socket_address, &boot.reason, boot.warp_out);

            // send reason
            let buf = build_unreliable_packet(&ServerPacket::Kick {
              reason: boot.reason.clone(),
            });

            let _ = socket.send_to(&buf, boot.socket_address);
          }

          net.tick();
        }
        ThreadMessage::ClientPacket {
          socket_address,
          headers,
          packet,
        } => {
          if headers.id == 0
            && headers.reliability.is_reliable()
            && !self.packet_sorter_map.contains_key(&socket_address)
          {
            // received the first reliable packet, store a new connection
            let packet_sorter = PacketSorter::new(socket_address);
            self.packet_sorter_map.insert(socket_address, packet_sorter);

            if self.config.log_connections {
              println!("{} connected", socket_address);
            }
          }

          if let Some(packet_sorter) = self.packet_sorter_map.get_mut(&socket_address) {
            let packets = packet_sorter.sort_packet(&socket, headers, packet);

            for packet in packets {
              self.handle_packet(&mut net, &socket, socket_address, packet);
            }
          } else {
            // ignoring errors, no packet sorter = never connected
            let _ = self.handle_packet(&mut net, &socket, socket_address, packet);
          }
        }
      }
    }
  }

  fn handle_packet(
    &mut self,
    net: &mut Net,
    socket: &std::net::UdpSocket,
    socket_address: std::net::SocketAddr,
    client_packet: ClientPacket,
  ) {
    if let Some(player_id) = self.player_id_map.get(&socket_address) {
      match client_packet {
        ClientPacket::Ping => {
          if self.config.log_packets {
            println!("Received bad Ping packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(&ServerPacket::Pong {
            max_payload_size: self.config.max_payload_size,
          });
          let _ = socket.send_to(&buf, socket_address);
        }
        ClientPacket::AssetFound {
          path,
          last_modified,
        } => {
          if self.config.log_packets {
            println!("Received AssetFound packet from {}", socket_address);
          }

          let mut is_valid = false;

          if let Some(asset) = net.get_asset(&path) {
            is_valid = asset.last_modified == last_modified;
          }

          if let Some(client) = net.get_client_mut(player_id) {
            if is_valid {
              client.cached_assets.insert(path);
            } else {
              client.packet_shipper.send(
                socket,
                Reliability::ReliableOrdered,
                &ServerPacket::RemoveAsset { path },
              );
            }
          }
        }
        ClientPacket::AssetStream { asset_type, data } => {
          if self.config.log_packets {
            println!("Received AssetStream packet from {}", socket_address);
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
            println!(
              "Received Ack for {:?} {} from {}",
              reliability, id, socket_address
            );
          }

          let client = net.get_client_mut(player_id).unwrap();
          client.packet_shipper.acknowledged(reliability, id);
        }
        ClientPacket::Login {
          username: _,
          data: _,
        } => {
          if self.config.log_packets {
            println!("Received bad Login packet from {}", socket_address);
          }
        }
        ClientPacket::RequestJoin => {
          if self.config.log_packets {
            println!("Received RequestJoin packet from {}", socket_address);
          }

          net.spawn_client(player_id);

          self.plugin_wrapper.handle_player_connect(net, player_id);

          net.connect_client(player_id);

          if self.config.log_connections {
            println!("{} connected", player_id);
          }
        }
        ClientPacket::Logout => {
          if self.config.log_packets {
            println!("Received Logout packet from {}", socket_address);
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
            println!("Received Position packet from {}", socket_address);
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
        ClientPacket::Ready => {
          if self.config.log_packets {
            println!("Received Ready packet from {}", socket_address);
          }

          use std::time::UNIX_EPOCH;

          let client = net.get_client_mut(player_id).unwrap();

          client.area_join_time = UNIX_EPOCH.elapsed().unwrap().as_millis() as u64;
          client.actor.x = client.warp_x;
          client.actor.y = client.warp_y;
          client.actor.z = client.warp_z;

          if client.transferring {
            self.plugin_wrapper.handle_player_transfer(net, &player_id);
          } else {
            self.plugin_wrapper.handle_player_join(net, &player_id);
          }

          net.mark_client_ready(player_id);
        }
        ClientPacket::TransferredOut => {
          if self.config.log_packets {
            println!("Received TransferredOut packet from {}", socket_address);
          }

          net.complete_transfer(player_id);
        }
        ClientPacket::CustomWarp { tile_object_id } => {
          if self.config.log_packets {
            println!("Received CustomWarp packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_custom_warp(net, &player_id, tile_object_id);
        }
        ClientPacket::AvatarChange => {
          if self.config.log_packets {
            println!("Received AvatarChange packet from {}", socket_address);
          }

          if let Some((texture_path, animation_path)) = net.store_player_assets(player_id) {
            let prevent_default = self.plugin_wrapper.handle_player_avatar_change(
              net,
              player_id,
              &texture_path,
              &animation_path,
            );

            if !prevent_default {
              net.set_player_avatar(player_id, texture_path, animation_path);
            }
          }
        }
        ClientPacket::Emote { emote_id } => {
          if self.config.log_packets {
            println!("Received Emote packet from {}", socket_address);
          }

          let prevent_default = self
            .plugin_wrapper
            .handle_player_emote(net, player_id, emote_id);

          if !prevent_default {
            net.set_player_emote(player_id, emote_id, false);
          }
        }
        ClientPacket::ObjectInteraction { tile_object_id } => {
          if self.config.log_packets {
            println!("Received ObjectInteraction packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_object_interaction(net, player_id, tile_object_id);
        }
        ClientPacket::ActorInteraction { actor_id } => {
          if self.config.log_packets {
            println!("Received ActorInteraction packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_actor_interaction(net, player_id, &actor_id);
        }
        ClientPacket::TileInteraction { x, y, z } => {
          if self.config.log_packets {
            println!("Received TileInteraction packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_tile_interaction(net, player_id, x, y, z);
        }
        ClientPacket::TextBoxResponse { response } => {
          if self.config.log_packets {
            println!("Received TextBoxResponse packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_textbox_response(net, player_id, response);
        }
        ClientPacket::PromptResponse { response } => {
          if self.config.log_packets {
            println!("Received PromptResponse packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_prompt_response(net, player_id, response);
        }
        ClientPacket::BoardOpen => {
          if self.config.log_packets {
            println!("Received BoardOpen packet from {}", socket_address);
          }

          self.plugin_wrapper.handle_board_open(net, player_id);
        }
        ClientPacket::BoardClose => {
          if self.config.log_packets {
            println!("Received BoardClose packet from {}", socket_address);
          }

          self.plugin_wrapper.handle_board_close(net, player_id);
        }
        ClientPacket::PostRequest => {
          if self.config.log_packets {
            println!("Received PostRequest packet from {}", socket_address);
          }

          self.plugin_wrapper.handle_post_request(net, player_id);
        }
        ClientPacket::PostSelection { post_id } => {
          if self.config.log_packets {
            println!("Received PostSelection packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_post_selection(net, player_id, &post_id);

          let client = net.get_client_mut(player_id).unwrap();

          client.packet_shipper.send(
            &socket,
            Reliability::ReliableOrdered,
            &ServerPacket::PostSelectionAck,
          );
        }
        ClientPacket::ServerMessage { data } => {
          // this should never happen but 🤷‍♂️
          if self.config.log_packets {
            println!("Received ServerMessage packet from {}", socket_address);
          }

          self
            .plugin_wrapper
            .handle_server_message(net, socket_address, &data);
        }
      }
    } else {
      match client_packet {
        ClientPacket::Ping => {
          if self.config.log_packets {
            println!("Received Ping packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(&ServerPacket::Pong {
            max_payload_size: self.config.max_payload_size,
          });
          let _ = socket.send_to(&buf, socket_address);
        }
        ClientPacket::Login { username, data } => {
          if self.config.log_packets {
            println!("Received Login packet from {}", socket_address);
          }

          let player_id = net.add_client(socket_address, username);

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
            println!("Received bad packet from {}", socket_address);
            println!("{:?}", client_packet);
            println!("Connected clients: {:?}", self.player_id_map.keys());
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
    if let Some(player_id) = self.player_id_map.remove(&socket_address) {
      self
        .plugin_wrapper
        .handle_player_disconnect(net, &player_id);

      net.remove_player(&player_id, warp_out);

      if self.config.log_connections {
        println!("{} disconnected for {}", player_id, reason);
      }
    }

    self.packet_sorter_map.remove(socket_address);

    if self.config.log_connections {
      println!("{} disconnected for {}", socket_address, reason);
    }
  }
}
