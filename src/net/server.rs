use super::Net;
use crate::packets::{
  build_unreliable_packet, ClientPacket, PacketSorter, Reliability, ServerPacket,
  MAX_PLAYER_ASSET_SIZE,
};
use crate::plugins::PluginInterface;
use crate::threads::{create_clock_thread, create_socket_thread, ThreadMessage};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct ServerConfig {
  pub port: u16,
  pub log_packets: bool,
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
      net: Net::new(rc_socket.clone()),
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
    create_socket_thread(tx, self.socket.try_clone()?, self.config.log_packets);

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

          // resend pending packets, kick anyone who had errors
          let mut kick_list = self.net.resend_backed_up_packets();

          // kick afk players
          let max_silence = std::time::Duration::from_secs(5);

          for (socket_address, packet_sorter) in &mut self.packet_sorter_map {
            let last_message = packet_sorter.get_last_message_time();

            if last_message.elapsed() > max_silence {
              kick_list.push(socket_address.clone())
            }
          }

          // actually kick players
          for socket_address in kick_list {
            self.disconnect_player(&socket_address);
          }
        }
        ThreadMessage::ClientPacket {
          socket_address,
          headers,
          packet,
        } => {
          if !matches!(headers.reliability, Reliability::Unreliable) {
            if !self.packet_sorter_map.contains_key(&socket_address) {
              let packet_sorter = PacketSorter::new(socket_address);
              self.packet_sorter_map.insert(socket_address, packet_sorter);
            }
          }

          if let Some(packet_sorter) = self.packet_sorter_map.get_mut(&socket_address) {
            if let Ok(packets) = packet_sorter.sort_packet(&self.socket, headers, packet) {
              for packet in packets {
                if let Err(_) = self.handle_packet(socket_address, packet) {
                  self.disconnect_player(&socket_address);
                  break;
                }
              }
            } else {
              self.disconnect_player(&socket_address);
            }
          } else {
            // ignoring errors, no packet sorter = never connected
            let _ = self.handle_packet(socket_address, packet);
          }
        }
      }

      self.net.broadcast_map_changes();
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

          let buf = build_unreliable_packet(&ServerPacket::Pong);
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::TextureStream { data } => {
          if self.config.log_packets {
            println!("Received Texture Stream packet from {}", socket_address);
          }

          append_texture_data(&mut self.player_texture_buffer, socket_address, data);
        }
        ClientPacket::AnimationStream { data } => {
          if self.config.log_packets {
            println!("Received Animation Stream packet from {}", socket_address);
          }

          append_texture_data(&mut self.player_animation_buffer, socket_address, data);
        }
        ClientPacket::Ack { reliability, id } => {
          if self.config.log_packets {
            println!(
              "Received Ack for {:?} {} from {}",
              reliability, id, socket_address
            );
          }

          let player = self.net.get_player_mut(player_id).unwrap();
          player.packet_shipper.acknowledged(reliability, id);
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

          self.disconnect_player(&socket_address);
        }
        ClientPacket::Position { x, y, z } => {
          if self.config.log_packets {
            println!("Received Position packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_move(&mut self.net, player_id, x, y, z);
          }

          self.net.move_player(player_id, x, y, z);
        }
        ClientPacket::Ready => {
          if self.config.log_packets {
            println!("Received Ready packet from {}", socket_address);
          }

          self.net.mark_player_ready(player_id);
        }
        ClientPacket::AvatarChange => {
          if self.config.log_packets {
            println!("Received Avatar Change packet from {}", socket_address);
          }

          let data_result = collect_streamed_player_data(
            &mut self.player_texture_buffer,
            &mut self.player_animation_buffer,
            &socket_address,
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

          self.net.set_player_emote(player_id, emote_id);
        }
      }
    } else {
      match client_packet {
        ClientPacket::Ping => {
          if self.config.log_packets {
            println!("Received Ping packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(&ServerPacket::Pong);
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::TextureStream { data } => {
          if self.config.log_packets {
            println!("Received Texture Stream packet from {}", socket_address);
          }

          append_texture_data(&mut self.player_texture_buffer, socket_address, data);
        }
        ClientPacket::AnimationStream { data } => {
          if self.config.log_packets {
            println!("Received Animation Stream packet from {}", socket_address);
          }

          append_texture_data(&mut self.player_animation_buffer, socket_address, data);
        }
        ClientPacket::Login {
          username,
          password: _,
        } => {
          if self.config.log_packets {
            println!("Received Login packet from {}", socket_address);
          }

          let data_result = collect_streamed_player_data(
            &mut self.player_texture_buffer,
            &mut self.player_animation_buffer,
            &socket_address,
          );

          if let Some((texture_data, animation_data)) = data_result {
            self.connect_player(socket_address, username, texture_data, animation_data)?;
          }
        }
        _ => {
          if self.config.log_packets {
            println!("Received bad packet from {}", socket_address);
            println!("{:?}", client_packet);
            println!("Connected players: {:?}", self.player_id_map.keys());
          }
        }
      }
    }

    Ok(())
  }

  fn connect_player(
    &mut self,
    socket_address: std::net::SocketAddr,
    name: String,
    texture_data: Vec<u8>,
    animation_data: String,
  ) -> std::io::Result<()> {
    let player_id =
      self
        .net
        .add_player(socket_address.clone(), name, texture_data, animation_data)?;

    for plugin in &mut self.plugin_interfaces {
      plugin.handle_player_connect(&mut self.net, &player_id);
    }

    self.net.connect_player(&player_id)?;
    self.player_id_map.insert(socket_address, player_id);

    Ok(())
  }

  fn disconnect_player(&mut self, socket_address: &std::net::SocketAddr) {
    if let Some(player_id) = self.player_id_map.remove(&socket_address) {
      for plugin in &mut self.plugin_interfaces {
        plugin.handle_player_disconnect(&mut self.net, &player_id);
      }

      self.net.remove_player(&player_id);
    }

    self.player_texture_buffer.remove(socket_address);
    self.player_animation_buffer.remove(socket_address);

    self.packet_sorter_map.remove(socket_address);
  }
}

fn append_texture_data(
  asset_buffer_map: &mut HashMap<std::net::SocketAddr, Vec<u8>>,
  socket_address: std::net::SocketAddr,
  data: Vec<u8>,
) {
  if let Some(buffer) = asset_buffer_map.get_mut(&socket_address) {
    if buffer.len() < MAX_PLAYER_ASSET_SIZE {
      buffer.extend(data);
    }
  } else {
    asset_buffer_map.insert(socket_address, data);
  }
}

fn collect_streamed_player_data(
  player_texture_buffer: &mut HashMap<std::net::SocketAddr, Vec<u8>>,
  player_animation_buffer: &mut HashMap<std::net::SocketAddr, Vec<u8>>,
  socket_address: &std::net::SocketAddr,
) -> Option<(Vec<u8>, String)> {
  let wrapped_texture_data = player_texture_buffer.remove(socket_address);
  let wrapped_animation_data = player_animation_buffer.remove(socket_address);

  let texture_data = wrapped_texture_data?;
  let animation_data = wrapped_animation_data?;

  if texture_data.len() > MAX_PLAYER_ASSET_SIZE || animation_data.len() > MAX_PLAYER_ASSET_SIZE {
    return None;
  }

  Some((texture_data, String::from_utf8(animation_data).ok()?))
}
