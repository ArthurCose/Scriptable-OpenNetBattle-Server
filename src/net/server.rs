use crate::net::Net;
use crate::net::Player;
use crate::packets::{
  build_unreliable_packet, ClientPacket, PacketShipper, PacketSorter, Reliability, ServerPacket,
};
use crate::plugins::{LuaPluginInterface, PluginInterface};
use crate::threads::{create_clock_thread, create_socket_thread, ThreadMessage};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

pub struct Server {
  player_id_map: HashMap<std::net::SocketAddr, String>,
  packet_sorter_map: HashMap<std::net::SocketAddr, PacketSorter>,
  net: Net,
  plugin_interfaces: Vec<Box<dyn PluginInterface>>,
  socket: Rc<UdpSocket>,
  log_packets: bool, // todo command line option
}

impl Server {
  pub fn new(port: u16) -> Server {
    let addr = format!("0.0.0.0:{}", port);
    let socket = UdpSocket::bind(addr).expect("Couldn't bind to address");

    match socket.take_error() {
      Ok(None) => println!("Server listening on: {}", port),
      Ok(Some(err)) => panic!("UdpSocket error: {:?}", err),
      Err(err) => panic!("UdpSocket.take_error failed: {:?}", err),
    }

    let rc_socket = Rc::new(socket);

    Server {
      player_id_map: HashMap::new(),
      packet_sorter_map: HashMap::new(),
      net: Net::new(rc_socket.clone()),
      plugin_interfaces: vec![Box::new(LuaPluginInterface::new())],
      socket: rc_socket,
      log_packets: false,
    }
  }

  pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::mpsc;
    use std::time::Instant;

    for plugin_interface in &mut self.plugin_interfaces {
      plugin_interface.init(&mut self.net);
    }

    let (tx, rx) = mpsc::channel();
    create_clock_thread(tx.clone());
    create_socket_thread(tx, self.socket.try_clone()?, self.log_packets);

    println!("Server started");

    let mut time = Instant::now();

    loop {
      match rx.recv()? {
        ThreadMessage::Tick(started) => {
          started();

          let elapsed_time = time.elapsed();
          time = Instant::now();

          for plugin in &mut self.plugin_interfaces {
            plugin.tick(&mut self.net, elapsed_time.as_secs_f64());
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
          match headers.reliability {
            Reliability::Unreliable => {
              // ignoring errors since pure unreliable packets are currently only used for ping + initial join
              let _ = self.handle_packet(socket_address, packet);
            }
            _ => {
              if !self.packet_sorter_map.contains_key(&socket_address) {
                let packet_sorter = PacketSorter::new(socket_address);
                self.packet_sorter_map.insert(socket_address, packet_sorter);
              }

              let packet_sorter = self.packet_sorter_map.get_mut(&socket_address).unwrap();

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
            }
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
          if self.log_packets {
            println!("Received bad Ping packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(&ServerPacket::Pong);
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::Ack { reliability, id } => {
          if self.log_packets {
            println!(
              "Received Ack for {:?} {} from {}",
              reliability, id, socket_address
            );
          }

          let player = self.net.get_player_mut(player_id).unwrap();
          player.packet_shipper.acknowledged(reliability, id);
        }
        ClientPacket::Login { username: _ } => {
          if self.log_packets {
            println!("Received second Login packet from {}", socket_address);
          }

          let packet = ServerPacket::Login {
            ticket: player_id.clone(),
            error: 0,
          };

          self
            .net
            .send_packet(player_id, &Reliability::Reliable, &packet)?;
        }
        ClientPacket::Logout => {
          if self.log_packets {
            println!("Received Logout packet from {}", socket_address);
          }

          self.disconnect_player(&socket_address);
        }
        ClientPacket::Position { x, y, z } => {
          if self.log_packets {
            println!("Received Position packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_move(&mut self.net, player_id, x, y, z);
          }

          self.net.move_player(player_id, x, y, z);
        }
        ClientPacket::LoadedMap { map_id: _ } => {
          if self.log_packets {
            println!("Received Map packet from {}", socket_address);
          }

          let player = self.net.get_player(player_id).unwrap();
          let area_id = &player.area_id.clone();
          let area = self.net.get_area(area_id).unwrap();

          // map signal
          let packet = ServerPacket::MapData {
            map_data: area.get_map().render(),
          };

          self
            .net
            .send_packet(player_id, &Reliability::ReliableOrdered, &packet)?;

          self.connect_player(&socket_address)?;
        }
        ClientPacket::AvatarChange { form_id } => {
          if self.log_packets {
            println!("Received Avatar Change packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_avatar_change(&mut self.net, player_id, form_id);
          }

          self.net.set_player_avatar(player_id, form_id);
        }
        ClientPacket::Emote { emote_id } => {
          if self.log_packets {
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
          if self.log_packets {
            println!("Received Ping packet from {}", socket_address);
          }

          let buf = build_unreliable_packet(&ServerPacket::Pong);
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::Login { username: _ } => {
          if self.log_packets {
            println!("Received Login packet from {}", socket_address);
          }

          self.add_player(socket_address)?;
        }
        _ => {
          if self.log_packets {
            println!("Received bad packet from {}", socket_address);
            println!("{:?}", client_packet);
            println!("Connected players: {:?}", self.player_id_map.keys());
          }
        }
      }
    }

    Ok(())
  }

  fn add_player(&mut self, socket_address: std::net::SocketAddr) -> std::io::Result<()> {
    use uuid::Uuid;

    let mut player = Player {
      socket_address,
      packet_shipper: PacketShipper::new(socket_address),
      area_id: self.net.get_default_area_id().clone(),
      id: Uuid::new_v4().to_string(),
      avatar_id: 0,
      x: 0.0,
      y: 0.0,
      z: 0.0,
      ready: false,
    };

    let packet = ServerPacket::Login {
      ticket: player.id.clone(),
      error: 0,
    };

    player
      .packet_shipper
      .send(&self.socket, &Reliability::Reliable, &packet)?;

    self.player_id_map.insert(socket_address, player.id.clone());
    self.net.add_player(player);

    Ok(())
  }

  fn connect_player(&mut self, socket_address: &std::net::SocketAddr) -> std::io::Result<()> {
    if let Some(player_id) = self.player_id_map.get_mut(&socket_address) {
      self.net.mark_player_ready(player_id);

      for plugin in &mut self.plugin_interfaces {
        plugin.handle_player_connect(&mut self.net, player_id);
      }

      let mut packets = vec![];

      for other_player in self.net.get_players() {
        packets.push(ServerPacket::NaviConnected {
          ticket: other_player.id.clone(),
        });

        packets.push(ServerPacket::NaviSetAvatar {
          ticket: other_player.id.clone(),
          avatar_id: other_player.avatar_id,
        });
      }

      for bot in self.net.get_bots() {
        packets.push(ServerPacket::NaviConnected {
          ticket: bot.id.clone(),
        });

        packets.push(ServerPacket::NaviSetAvatar {
          ticket: bot.id.clone(),
          avatar_id: bot.avatar_id,
        });
      }

      let player = self.net.get_player_mut(player_id).unwrap();

      for packet in packets {
        player
          .packet_shipper
          .send(&self.socket, &Reliability::ReliableOrdered, &packet)?;
      }
    }

    Ok(())
  }

  fn disconnect_player(&mut self, socket_address: &std::net::SocketAddr) {
    if let Some(player_id) = self.player_id_map.remove(&socket_address) {
      for plugin in &mut self.plugin_interfaces {
        plugin.handle_player_disconnect(&mut self.net, &player_id);
      }

      self.net.remove_player(&player_id);
    }

    self.packet_sorter_map.remove(socket_address);
  }
}
