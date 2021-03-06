use super::Net;
use crate::packets::{
  build_unreliable_packet, ClientPacket, PacketSorter, Reliability, ServerPacket,
};
use crate::plugins::PluginInterface;
use crate::threads::{create_clock_thread, create_socket_thread, ThreadMessage};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct ServerConfig {
  pub port: u16,
  pub max_payload_size: usize,
  pub log_connections: bool,
  pub log_packets: bool,
  pub player_asset_limit: usize,
}

pub struct Server {
  player_texture_buffer: HashMap<std::net::SocketAddr, Vec<u8>>,
  player_animation_buffer: HashMap<std::net::SocketAddr, Vec<u8>>,
  player_id_map: HashMap<std::net::SocketAddr, String>,
  packet_sorter_map: HashMap<std::net::SocketAddr, PacketSorter>,
  net: Net,
  plugin_interfaces: Vec<Box<dyn PluginInterface>>,
  socket: Rc<UdpSocket>,
  config: ServerConfig,
}

impl Server {
  pub fn new(config: ServerConfig) -> Server {
    let addr = format!("0.0.0.0:{}", config.port);
    let socket = UdpSocket::bind(addr).expect("Couldn't bind to address");

    match socket.take_error() {
      Ok(None) => println!("Server listening on: {}", config.port),
      Ok(Some(err)) => panic!("UdpSocket error: {:?}", err),
      Err(err) => panic!("UdpSocket.take_error failed: {:?}", err),
    }

    let rc_socket = Rc::new(socket);

    Server {
      player_texture_buffer: HashMap::new(),
      player_animation_buffer: HashMap::new(),
      player_id_map: HashMap::new(),
      packet_sorter_map: HashMap::new(),
      net: Net::new(rc_socket.clone(), config.max_payload_size),
      plugin_interfaces: Vec::new(),
      socket: rc_socket,
      config,
    }
  }

  pub fn add_plugin_interface(&mut self, plugin_interface: Box<dyn PluginInterface>) {
    self.plugin_interfaces.push(plugin_interface);
  }

  pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::mpsc;
    use std::time::Instant;

    for plugin_interface in &mut self.plugin_interfaces {
      plugin_interface.init(&mut self.net);
    }

    let (tx, rx) = mpsc::channel();
    create_clock_thread(tx.clone());
    create_socket_thread(
      tx,
      self.socket.try_clone()?,
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

          for plugin in &mut self.plugin_interfaces {
            plugin.tick(&mut self.net, elapsed_time.as_secs_f32());
          }

          // kick afk clients
          let mut kick_list = Vec::new();
          let max_silence = std::time::Duration::from_secs(5);

          for (socket_address, packet_sorter) in &mut self.packet_sorter_map {
            let last_message = packet_sorter.get_last_message_time();

            if last_message.elapsed() > max_silence {
              kick_list.push(*socket_address)
            }
          }

          // actually kick clients
          for socket_address in kick_list {
            self.disconnect_client(&socket_address);
          }

          self.net.tick();
        }
        ThreadMessage::ClientPacket {
          socket_address,
          headers,
          packet,
        } => {
          if !matches!(headers.reliability, Reliability::Unreliable)
            && !self.packet_sorter_map.contains_key(&socket_address)
          {
            let packet_sorter = PacketSorter::new(socket_address);
            self.packet_sorter_map.insert(socket_address, packet_sorter);

            if self.config.log_connections {
              println!("{} connected", socket_address);
            }
          }

          if let Some(packet_sorter) = self.packet_sorter_map.get_mut(&socket_address) {
            if let Ok(packets) = packet_sorter.sort_packet(&self.socket, headers, packet) {
              for packet in packets {
                if self.handle_packet(socket_address, packet).is_err() {
                  self.disconnect_client(&socket_address);
                  break;
                }
              }
            } else {
              self.disconnect_client(&socket_address);
            }
          } else {
            // ignoring errors, no packet sorter = never connected
            let _ = self.handle_packet(socket_address, packet);
          }
        }
      }
    }
  }

  fn handle_packet(
    &mut self,
    socket_address: std::net::SocketAddr,
    client_packet: ClientPacket,
  ) -> std::io::Result<()> {
    if let Some(player_id) = self.player_id_map.get(&socket_address) {
      match client_packet {
        ClientPacket::Ping => {
          if self.config.log_packets {
            println!("Received bad Ping packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(&ServerPacket::Pong {
            max_payload_size: self.config.max_payload_size,
          });
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::TextureStream { data } => {
          if self.config.log_packets {
            println!("Received Texture Stream packet from {}", socket_address);
          }

          append_texture_data(
            &mut self.player_texture_buffer,
            socket_address,
            data,
            self.config.player_asset_limit,
          );
        }
        ClientPacket::AnimationStream { data } => {
          if self.config.log_packets {
            println!("Received Animation Stream packet from {}", socket_address);
          }

          append_texture_data(
            &mut self.player_animation_buffer,
            socket_address,
            data,
            self.config.player_asset_limit,
          );
        }
        ClientPacket::Ack { reliability, id } => {
          if self.config.log_packets {
            println!(
              "Received Ack for {:?} {} from {}",
              reliability, id, socket_address
            );
          }

          let client = self.net.get_client_mut(player_id).unwrap();
          client.packet_shipper.acknowledged(reliability, id);
        }
        ClientPacket::Login {
          username: _,
          password: _,
        } => {
          if self.config.log_packets {
            println!("Received bad Login packet from {}", socket_address);
          }
        }
        ClientPacket::Logout => {
          if self.config.log_packets {
            println!("Received Logout packet from {}", socket_address);
          }

          self.disconnect_client(&socket_address);
        }
        ClientPacket::Position { x, y, z } => {
          if self.config.log_packets {
            println!("Received Position packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_move(&mut self.net, player_id, x, y, z);
          }

          self.net.update_player_position(player_id, x, y, z);
        }
        ClientPacket::Ready => {
          if self.config.log_packets {
            println!("Received Ready packet from {}", socket_address);
          }

          let client = self.net.get_client(player_id).unwrap();

          // if the client is ready, this is a transfer
          if client.ready {
            for plugin in &mut self.plugin_interfaces {
              plugin.handle_player_transfer(&mut self.net, &player_id);
            }
          }

          self.net.mark_client_ready(player_id);
        }
        ClientPacket::AvatarChange => {
          if self.config.log_packets {
            println!("Received Avatar Change packet from {}", socket_address);
          }

          let data_result = collect_streamed_client_data(
            &mut self.player_texture_buffer,
            &mut self.player_animation_buffer,
            &socket_address,
            self.config.player_asset_limit,
          );

          if let Some((texture_data, animation_data)) = data_result {
            let (texture_path, animation_path) =
              self
                .net
                .store_player_avatar(player_id, texture_data, animation_data);

            for plugin in &mut self.plugin_interfaces {
              plugin.handle_player_avatar_change(
                &mut self.net,
                player_id,
                &texture_path,
                &animation_path,
              );
            }
            self
              .net
              .set_player_avatar(player_id, texture_path, animation_path);
          }
        }
        ClientPacket::Emote { emote_id } => {
          if self.config.log_packets {
            println!("Received Emote packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_emote(&mut self.net, player_id, emote_id);
          }

          self.net.player_emote(player_id, emote_id);
        }
        ClientPacket::ObjectInteraction { tile_object_id } => {
          if self.config.log_packets {
            println!("Received ObjectInteraction packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_object_interaction(&mut self.net, player_id, tile_object_id);
          }
        }
        ClientPacket::NaviInteraction { navi_id } => {
          if self.config.log_packets {
            println!("Received ObjectInteraction packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_navi_interaction(&mut self.net, player_id, &navi_id);
          }
        }
        ClientPacket::TileInteraction { x, y, z } => {
          if self.config.log_packets {
            println!("Received TileInteraction packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_tile_interaction(&mut self.net, player_id, x, y, z);
          }
        }
        ClientPacket::DialogResponse { response } => {
          if self.config.log_packets {
            println!("Received Dialog Response packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_dialog_response(&mut self.net, player_id, response);
          }
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
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::TextureStream { data } => {
          if self.config.log_packets {
            println!("Received Texture Stream packet from {}", socket_address);
          }

          append_texture_data(
            &mut self.player_texture_buffer,
            socket_address,
            data,
            self.config.player_asset_limit,
          );
        }
        ClientPacket::AnimationStream { data } => {
          if self.config.log_packets {
            println!("Received Animation Stream packet from {}", socket_address);
          }

          append_texture_data(
            &mut self.player_animation_buffer,
            socket_address,
            data,
            self.config.player_asset_limit,
          );
        }
        ClientPacket::Login {
          username,
          password: _,
        } => {
          if self.config.log_packets {
            println!("Received Login packet from {}", socket_address);
          }

          let data_result = collect_streamed_client_data(
            &mut self.player_texture_buffer,
            &mut self.player_animation_buffer,
            &socket_address,
            self.config.player_asset_limit,
          );

          if let Some((texture_data, animation_data)) = data_result {
            self.connect_client(socket_address, username, texture_data, animation_data);
          }
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

    Ok(())
  }

  fn connect_client(
    &mut self,
    socket_address: std::net::SocketAddr,
    name: String,
    texture_data: Vec<u8>,
    animation_data: String,
  ) {
    let player_id = self
      .net
      .add_player(socket_address, name, texture_data, animation_data);

    for plugin in &mut self.plugin_interfaces {
      plugin.handle_player_connect(&mut self.net, &player_id);
    }

    self.net.connect_client(&player_id);

    if self.config.log_connections {
      println!("{} connected", player_id);
    }

    self.player_id_map.insert(socket_address, player_id);
  }

  fn disconnect_client(&mut self, socket_address: &std::net::SocketAddr) {
    if let Some(player_id) = self.player_id_map.remove(&socket_address) {
      for plugin in &mut self.plugin_interfaces {
        plugin.handle_player_disconnect(&mut self.net, &player_id);
      }

      self.net.remove_player(&player_id);

      if self.config.log_connections {
        println!("{} disconnected", player_id);
      }
    }

    self.player_texture_buffer.remove(socket_address);
    self.player_animation_buffer.remove(socket_address);

    self.packet_sorter_map.remove(socket_address);

    if self.config.log_connections {
      println!("{} disconnected", socket_address);
    }
  }
}

fn append_texture_data(
  asset_buffer_map: &mut HashMap<std::net::SocketAddr, Vec<u8>>,
  socket_address: std::net::SocketAddr,
  data: Vec<u8>,
  player_asset_limit: usize,
) {
  if let Some(buffer) = asset_buffer_map.get_mut(&socket_address) {
    if buffer.len() < player_asset_limit {
      buffer.extend(data);
    }
  } else {
    asset_buffer_map.insert(socket_address, data);
  }
}

fn collect_streamed_client_data(
  player_texture_buffer: &mut HashMap<std::net::SocketAddr, Vec<u8>>,
  player_animation_buffer: &mut HashMap<std::net::SocketAddr, Vec<u8>>,
  socket_address: &std::net::SocketAddr,
  player_asset_limit: usize,
) -> Option<(Vec<u8>, String)> {
  let wrapped_texture_data = player_texture_buffer.remove(socket_address);
  let wrapped_animation_data = player_animation_buffer.remove(socket_address);

  let texture_data = wrapped_texture_data?;
  let animation_data = wrapped_animation_data?;

  if texture_data.len() > player_asset_limit || animation_data.len() > player_asset_limit {
    println!("{} player assets too large", socket_address);
    return None;
  }

  Some((texture_data, String::from_utf8(animation_data).ok()?))
}
