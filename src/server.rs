use crate::net::Net;
use crate::net::Player;
use crate::packets::{build_packet, ClientPacket, ServerPacket};
use crate::plugins::{LuaPluginInterface, PluginInterface};
use crate::threads::{create_clock_thread, create_socket_thread, ThreadMessage};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

const OBN_PORT: usize = 8765;

pub struct Server {
  player_id_map: HashMap<std::net::SocketAddr, String>,
  net: Net,
  plugin_interfaces: Vec<Box<dyn PluginInterface>>,
  socket: Rc<UdpSocket>,
  log_packets: bool, // todo command line option
}

impl Server {
  pub fn new() -> Server {
    let addr = format!("0.0.0.0:{}", OBN_PORT);
    let socket = UdpSocket::bind(addr).expect("Couldn't bind to address");

    match socket.take_error() {
      Ok(None) => println!("Server listening on: {}", OBN_PORT),
      Ok(Some(err)) => panic!("UdpSocket error: {:?}", err),
      Err(err) => panic!("UdpSocket.take_error failed: {:?}", err),
    }

    let rc_socket = Rc::new(socket);

    Server {
      player_id_map: HashMap::new(),
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
        }
        ThreadMessage::ClientPacket(src_addr, client_packet) => {
          // ignoring errors
          let _ = self.handle_packet(src_addr, client_packet);
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

          let buf = build_packet(ServerPacket::Pong);
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::Login { username: _ } => {
          if self.log_packets {
            println!("Received second Login packet from {}", socket_address);
          }

          let buf = build_packet(ServerPacket::Login {
            ticket: player_id.clone(),
            error: 0,
          });

          self.socket.send_to(&buf, socket_address)?;
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

          let area_id = &self.net.get_player(player_id).unwrap().area_id.clone();
          let area = self.net.get_area(area_id).unwrap();

          // map signal
          let buf = build_packet(ServerPacket::MapData {
            map_data: area.get_map().render(),
          });

          self.socket.send_to(&buf, socket_address)?;

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

          let buf = build_packet(ServerPacket::Pong);
          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::Login { username: _ } => {
          if self.log_packets {
            println!("Received Login packet from {}", socket_address);
          }

          self.add_player(socket_address);

          // login packet
          if let Some(player_id) = self.player_id_map.get(&socket_address) {
            let buf = build_packet(ServerPacket::Login {
              ticket: player_id.clone(),
              error: 0,
            });

            self.socket.send_to(&buf, socket_address)?;
          }
        }
        _ => {
          if self.log_packets {
            println!("Received bad packet from {}", socket_address,);
            println!("{:?}", client_packet);
            println!("Connected players: {:?}", self.player_id_map.keys());
          }
        }
      }
    }

    Ok(())
  }

  fn add_player(&mut self, socket_address: std::net::SocketAddr) {
    use uuid::Uuid;

    let player = Player {
      socket_address,
      area_id: self.net.get_default_area_id().clone(),
      id: Uuid::new_v4().to_string(),
      avatar_id: 0,
      x: 0.0,
      y: 0.0,
      z: 0.0,
      ready: false,
    };

    self.player_id_map.insert(socket_address, player.id.clone());

    self.net.add_player(player);
  }

  fn connect_player(&mut self, socket_address: &std::net::SocketAddr) -> std::io::Result<()> {
    if let Some(player_id) = self.player_id_map.get_mut(&socket_address) {
      self.net.mark_player_ready(player_id);

      for plugin in &mut self.plugin_interfaces {
        plugin.handle_player_connect(&mut self.net, player_id);
      }
    }

    for other_player in self.net.get_players() {
      let buf = build_packet(ServerPacket::NaviConnected {
        ticket: other_player.id.clone(),
      });
      self.socket.send_to(&buf, &socket_address)?;

      // trigger player spawning by sending position
      let buf = build_packet(ServerPacket::NaviWalkTo {
        ticket: other_player.id.clone(),
        x: other_player.x,
        y: other_player.y,
        z: other_player.z,
      });
      self.socket.send_to(&buf, &socket_address)?;

      // update avatar
      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: other_player.id.clone(),
        avatar_id: other_player.avatar_id,
      });
      self.socket.send_to(&buf, &socket_address)?;
    }

    for bot in self.net.get_bots() {
      let buf = build_packet(ServerPacket::NaviConnected {
        ticket: bot.id.clone(),
      });
      self.socket.send_to(&buf, &socket_address)?;

      // trigger player spawning by sending position
      let buf = build_packet(ServerPacket::NaviWalkTo {
        ticket: bot.id.clone(),
        x: bot.x,
        y: bot.y,
        z: bot.z,
      });
      self.socket.send_to(&buf, &socket_address)?;

      // update avatar
      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: bot.id.clone(),
        avatar_id: bot.avatar_id,
      });
      self.socket.send_to(&buf, &socket_address)?;
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
  }
}
