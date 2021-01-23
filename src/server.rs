use crate::area::Area;
use crate::area::Map;
use crate::packets::{build_packet, ClientPacket, ServerPacket};
use crate::player::Player;
use crate::plugins::{LuaPluginInterface, PluginInterface};
use crate::threads::{create_clock_thread, create_socket_thread, ThreadMessage};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;

const OBN_PORT: usize = 8765;

pub struct Server {
  player_id_map: HashMap<std::net::SocketAddr, String>,
  area: Area,
  plugin_interfaces: Vec<Box<dyn PluginInterface>>,
  socket: Rc<UdpSocket>,
  log_packets: bool, // todo command line option
}

impl Server {
  pub fn new() -> Server {
    let addr = format!("127.0.0.1:{}", OBN_PORT);
    let socket = UdpSocket::bind(addr).expect("Couldn't bind to address");

    match socket.take_error() {
      Ok(None) => println!("Server listening on: {}", OBN_PORT),
      Ok(Some(err)) => panic!("UdpSocket error: {:?}", err),
      Err(err) => panic!("UdpSocket.take_error failed: {:?}", err),
    }

    let rc_socket = Rc::new(socket);

    let map_string = std::fs::read_to_string("map.txt").expect("Failed to read map.txt");

    Server {
      player_id_map: HashMap::new(),
      area: Area::new(rc_socket.clone(), Map::from(String::from(map_string))),
      plugin_interfaces: vec![Box::new(LuaPluginInterface::new())],
      socket: rc_socket,
      log_packets: false,
    }
  }

  pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::mpsc;
    use std::time::Instant;

    for plugin_interface in &mut self.plugin_interfaces {
      plugin_interface.init(&mut self.area);
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
            plugin.tick(&mut self.area, elapsed_time.as_secs_f64());
          }
        }
        ThreadMessage::ClientPacket(src_addr, client_packet) => {
          // ignoring errors
          let _ = self.handle_packet(src_addr, client_packet);
        }
      }

      // todo: handle possible errors
      let _ = self.area.broadcast_map_changes();
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

          self.connect_player(&socket_address)?;
        }
        ClientPacket::Logout => {
          if self.log_packets {
            println!("Received Logout packet from {}", socket_address);
          }

          self.disconnect_player(&socket_address)?;
        }
        ClientPacket::Position { x, y, z } => {
          if self.log_packets {
            println!("Received Position packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_move(&mut self.area, player_id, x, y, z);
          }

          self.area.move_player(player_id, x, y, z)?;
        }
        ClientPacket::LoadedMap { map_id: _ } => {
          if self.log_packets {
            println!("Received Map packet from {}", socket_address);
          }

          // map signal
          let buf = build_packet(ServerPacket::MapData {
            map_data: self.area.get_map().render(),
          });

          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::AvatarChange { form_id } => {
          if self.log_packets {
            println!("Received Avatar Change packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_avatar_change(&mut self.area, player_id, form_id);
          }

          self.area.set_player_avatar(player_id, form_id)?;
        }
        ClientPacket::Emote { emote_id } => {
          if self.log_packets {
            println!("Received Emote packet from {}", socket_address);
          }

          for plugin in &mut self.plugin_interfaces {
            plugin.handle_player_emote(&mut self.area, player_id, emote_id);
          }

          self.area.set_player_emote(player_id, emote_id)?;
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

          self.add_player(socket_address)?;

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

  fn add_player(&mut self, socket_address: std::net::SocketAddr) -> std::io::Result<()> {
    use uuid::Uuid;

    let player = Player {
      socket_address,
      ticket: Uuid::new_v4().to_string(),
      avatar_id: 0,
      x: 0.0,
      y: 0.0,
      z: 0.0,
      ready: false,
    };

    self
      .player_id_map
      .insert(socket_address, player.ticket.clone());

    self.area.add_player(player)?;

    Ok(())
  }

  fn connect_player(&mut self, socket_address: &std::net::SocketAddr) -> std::io::Result<()> {
    if let Some(player_id) = self.player_id_map.get_mut(&socket_address) {
      self.area.mark_player_ready(player_id);

      let buf = build_packet(ServerPacket::Login {
        ticket: player_id.clone(),
        error: 0,
      });

      self.socket.send_to(&buf, socket_address)?;

      for plugin in &mut self.plugin_interfaces {
        plugin.handle_player_join(&mut self.area, player_id);
      }
    }

    for other_player in self.area.get_players() {
      let buf = build_packet(ServerPacket::NaviConnected {
        ticket: other_player.ticket.clone(),
      });
      self.socket.send_to(&buf, &socket_address)?;

      // trigger player spawning by sending position
      let buf = build_packet(ServerPacket::NaviWalkTo {
        ticket: other_player.ticket.clone(),
        x: other_player.x,
        y: other_player.y,
        z: other_player.z,
      });
      self.socket.send_to(&buf, &socket_address)?;

      // update avatar
      let buf = build_packet(ServerPacket::NaviSetAvatar {
        ticket: other_player.ticket.clone(),
        avatar_id: other_player.avatar_id,
      });
      self.socket.send_to(&buf, &socket_address)?;
    }

    for bot in self.area.get_bots() {
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

  fn disconnect_player(&mut self, socket_address: &std::net::SocketAddr) -> std::io::Result<()> {
    if let Some(player_id) = self.player_id_map.remove(&socket_address) {
      self.area.remove_player(&player_id)?;

      for plugin in &mut self.plugin_interfaces {
        plugin.handle_player_disconnect(&mut self.area, &player_id);
      }
    }

    Ok(())
  }
}
