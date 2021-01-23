use crate::map::Map;
use crate::packets::{build_packet, ClientPacket, ServerPacket};
use crate::plugins::{LuaPlugin, Plugin};
use crate::threads::{create_clock_thread, create_socket_thread, ThreadMessage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::UdpSocket;

const OBN_PORT: usize = 8765;

pub struct Server {
  players: HashMap<std::net::SocketAddr, Player>,
  map: RefCell<Map>,
  plugins: Vec<Box<dyn Plugin>>,
  socket: UdpSocket,
  log_packets: bool, // todo command line option
}

struct Player {
  socket_address: std::net::SocketAddr,
  ticket: String,
  avatar_id: u16,
  x: f64,
  y: f64,
  z: f64,
  ready: bool,
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

    let bytes = include_bytes!("../map.txt");
    let map_str = std::str::from_utf8(bytes).unwrap();

    Server {
      players: HashMap::new(),
      map: RefCell::new(Map::from(String::from(map_str))),
      plugins: vec![Box::new(LuaPlugin::new())],
      socket,
      log_packets: false,
    }
  }

  pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::mpsc;
    use std::time::Instant;

    let (tx, rx) = mpsc::channel();
    create_clock_thread(tx.clone());
    create_socket_thread(tx, self.socket.try_clone()?, self.log_packets);

    println!("Server started");

    let mut time = Instant::now();

    loop {
      match rx.recv()? {
        ThreadMessage::Tick => {
          let elapsed_time = time.elapsed();
          time = Instant::now();

          for plugin in &mut self.plugins {
            plugin.tick(&mut self.map, elapsed_time.as_secs_f64());
          }
        }
        ThreadMessage::ClientPacket(src_addr, client_packet) => {
          // ignoring errors
          let _ = self.handle_packet(src_addr, client_packet);
        }
      }

      let mut map = self.map.borrow_mut();

      if map.is_dirty() {
        let buf = build_packet(ServerPacket::MapData {
          map_data: map.render(),
        });

        let _ = self.broadcast(&buf);
      }
    }
  }

  fn handle_packet(
    &mut self,
    socket_address: std::net::SocketAddr,
    client_packet: ClientPacket,
  ) -> std::io::Result<()> {
    if self.players.contains_key(&socket_address) {
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

          let player = self.players.get_mut(&socket_address).unwrap();
          player.x = x;
          player.y = y;
          player.z = z;

          for plugin in &mut self.plugins {
            plugin.handle_player_move(&mut self.map, player.ticket.clone(), x, y, z);
          }

          let buf = build_packet(ServerPacket::NaviWalkTo {
            ticket: player.ticket.clone(),
            x,
            y,
            z,
          });

          self.broadcast(&buf)?;
        }
        ClientPacket::LoadedMap { map_id: _ } => {
          if self.log_packets {
            println!("Received Map packet from {}", socket_address);
          }

          // map signal
          let buf = build_packet(ServerPacket::MapData {
            map_data: self.map.borrow_mut().render(),
          });

          self.socket.send_to(&buf, socket_address)?;
        }
        ClientPacket::AvatarChange { form_id } => {
          if self.log_packets {
            println!("Received Avatar Change packet from {}", socket_address);
          }

          let player = self.players.get_mut(&socket_address).unwrap();
          player.avatar_id = form_id;

          for plugin in &mut self.plugins {
            plugin.handle_player_avatar_change(&mut self.map, player.ticket.clone(), form_id);
          }

          let buf = build_packet(ServerPacket::NaviSetAvatar {
            ticket: player.ticket.clone(),
            avatar_id: form_id,
          });

          self.broadcast(&buf)?;
        }
        ClientPacket::Emote { emote_id } => {
          if self.log_packets {
            println!("Received Emote packet from {}", socket_address);
          }

          let player = self.players.get(&socket_address).unwrap();

          for plugin in &mut self.plugins {
            plugin.handle_player_emote(&mut self.map, player.ticket.clone(), emote_id);
          }

          let buf = build_packet(ServerPacket::NaviEmote {
            ticket: player.ticket.clone(),
            emote_id,
          });

          self.broadcast(&buf)?;
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
          if let Some(player) = self.players.get(&socket_address) {
            let buf = build_packet(ServerPacket::Login {
              ticket: player.ticket.clone(),
              error: 0,
            });

            self.socket.send_to(&buf, socket_address)?;
          }
        }
        _ => {
          if self.log_packets {
            println!("Received bad packet from {}", socket_address,);
            println!("{:?}", client_packet);
            println!("Connected players: {:?}", self.players.keys());
          }
        }
      }
    }

    Ok(())
  }

  fn broadcast(&self, buf: &[u8]) -> std::io::Result<()> {
    for player in self.players.values() {
      if player.ready {
        self.socket.send_to(buf, player.socket_address)?;
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

    // player join packet
    let buf = build_packet(ServerPacket::NaviConnected {
      ticket: player.ticket.clone(),
    });
    self.broadcast(&buf)?;

    self.players.insert(socket_address, player);

    Ok(())
  }

  fn connect_player(&mut self, socket_address: &std::net::SocketAddr) -> std::io::Result<()> {
    if let Some(player) = self.players.get_mut(&socket_address) {
      player.ready = true;

      let buf = build_packet(ServerPacket::Login {
        ticket: player.ticket.clone(),
        error: 0,
      });

      self.socket.send_to(&buf, socket_address)?;

      for plugin in &mut self.plugins {
        plugin.handle_player_join(&mut self.map, player.ticket.clone());
      }
    }

    for other_player in self.players.values() {
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

    Ok(())
  }

  fn disconnect_player(&mut self, socket_address: &std::net::SocketAddr) -> std::io::Result<()> {
    if let Some(player) = self.players.remove(&socket_address) {
      let buf = build_packet(ServerPacket::NaviDisconnected {
        ticket: player.ticket.clone(),
      });

      self.broadcast(&buf)?;

      for plugin in &mut self.plugins {
        plugin.handle_player_disconnect(&mut self.map, player.ticket.clone());
      }
    }

    Ok(())
  }
}
