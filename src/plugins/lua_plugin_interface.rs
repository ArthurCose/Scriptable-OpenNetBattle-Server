use super::plugin_interface::PluginInterface;
use crate::net::{Bot, Net};
use paste::paste;
use rlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct LuaPluginInterface {
  scripts: HashMap<std::path::PathBuf, Lua>,
  tick_listeners: Vec<std::path::PathBuf>,
  player_join_listeners: Vec<std::path::PathBuf>,
  player_disconnect_listeners: Vec<std::path::PathBuf>,
  player_move_listeners: Vec<std::path::PathBuf>,
  player_avatar_change_listeners: Vec<std::path::PathBuf>,
  player_emote_listeners: Vec<std::path::PathBuf>,
}

fn create_area_error(id: &String) -> rlua::Error {
  rlua::Error::RuntimeError(String::from(format!("No area matching \"{}\" found.", id)))
}

fn create_bot_error(id: &String) -> rlua::Error {
  rlua::Error::RuntimeError(String::from(format!("No bot matching \"{}\" found.", id)))
}

fn create_player_error(id: &String) -> rlua::Error {
  rlua::Error::RuntimeError(String::from(format!(
    "No player matching \"{}\" found.",
    id
  )))
}

// abusing macro to auto resolve lifetime
macro_rules! create_area_table {
  ($lua_ctx: ident, $scope: ident, $net_ref: ident) => {{
    let area_table = $lua_ctx.create_table()?;

    area_table.set(
      "get_default_area",
      $scope.create_function(|_, ()| {
        let net = $net_ref.borrow();

        Ok(net.get_default_area_id().clone())
      })?,
    )?;

    area_table.set(
      "get_width",
      $scope.create_function(|_, area_id: String| {
        let mut net = $net_ref.borrow_mut();

        if let Some(area) = net.get_area(&area_id) {
          Ok(area.get_map().get_width())
        } else {
          Err(create_area_error(&area_id))
        }
      })?,
    )?;

    area_table.set(
      "get_height",
      $scope.create_function(|_, area_id: String| {
        let mut net = $net_ref.borrow_mut();

        if let Some(area) = net.get_area(&area_id) {
          Ok(area.get_map().get_height())
        } else {
          Err(create_area_error(&area_id))
        }
      })?,
    )?;

    area_table.set(
      "get_tile",
      $scope.create_function(|_, (area_id, x, y): (String, usize, usize)| {
        let mut net = $net_ref.borrow_mut();

        if let Some(area) = net.get_area(&area_id) {
          Ok(area.get_map().get_tile(x, y))
        } else {
          Err(create_area_error(&area_id))
        }
      })?,
    )?;

    area_table.set(
      "set_tile",
      $scope.create_function(|_, (area_id, x, y, id): (String, usize, usize, String)| {
        let mut net = $net_ref.borrow_mut();

        if let Some(area) = net.get_area(&area_id) {
          Ok(area.get_map().set_tile(x, y, id))
        } else {
          Err(create_area_error(&area_id))
        }
      })?,
    )?;

    area_table
  }};
}

macro_rules! create_bot_table {
  ($lua_ctx: ident, $scope: ident, $net_ref: ident) => {{
    let bot_table = $lua_ctx.create_table()?;

    bot_table.set(
      "list_bots",
      $scope.create_function(|_, area_id: String| {
        let mut net = $net_ref.borrow_mut();

        if let Some(area) = net.get_area(&area_id) {
          let connected_bots = area.get_connected_bots();
          let result: Vec<String> = connected_bots.iter().map(|id| id.clone()).collect();

          Ok(result)
        } else {
          Err(create_area_error(&area_id))
        }
      })?,
    )?;

    bot_table.set(
      "create_bot",
      $scope.create_function(
        |_, (id, area_id, avatar_id, x, y, z): (String, String, u16, f64, f64, f64)| {
          let mut net = $net_ref.borrow_mut();

          if let Some(_) = net.get_area(&area_id) {
            let bot = Bot {
              id,
              area_id,
              avatar_id,
              x,
              y,
              z,
            };

            net.add_bot(bot);

            Ok(())
          } else {
            Err(create_area_error(&id))
          }
        },
      )?,
    )?;

    bot_table.set(
      "is_bot",
      $scope.create_function(|_, id: String| {
        let net = $net_ref.borrow();

        if let Some(_) = net.get_bot(&id) {
          Ok(true)
        } else {
          Ok(false)
        }
      })?,
    )?;

    bot_table.set(
      "remove_bot",
      $scope.create_function(|_, id: String| {
        let mut net = $net_ref.borrow_mut();

        net.remove_bot(&id);

        Ok(())
      })?,
    )?;

    bot_table.set(
      "get_bot_area",
      $scope.create_function(|_, id: String| {
        let net = $net_ref.borrow_mut();

        if let Some(bot) = net.get_bot(&id) {
          Ok(bot.area_id.clone())
        } else {
          Err(create_bot_error(&id))
        }
      })?,
    )?;

    bot_table.set(
      "get_bot_position",
      $scope.create_function(|lua_ctx, id: String| {
        let net = $net_ref.borrow();

        if let Some(bot) = net.get_bot(&id) {
          let table = lua_ctx.create_table()?;
          table.set("x", bot.x)?;
          table.set("y", bot.y)?;
          table.set("z", bot.z)?;

          Ok(table)
        } else {
          Err(create_bot_error(&id))
        }
      })?,
    )?;

    bot_table.set(
      "move_bot",
      $scope.create_function(|_, (id, x, y, z): (String, f64, f64, f64)| {
        let mut net = $net_ref.borrow_mut();

        net.move_bot(&id, x, y, z);

        Ok(())
      })?,
    )?;

    bot_table.set(
      "set_bot_avatar",
      $scope.create_function(|_, (id, avatar_id): (String, u16)| {
        let mut net = $net_ref.borrow_mut();

        net.set_bot_avatar(&id, avatar_id);

        Ok(())
      })?,
    )?;

    bot_table.set(
      "set_bot_emote",
      $scope.create_function(|_, (id, emote_id): (String, u8)| {
        let mut net = $net_ref.borrow_mut();

        net.set_bot_emote(&id, emote_id);

        Ok(())
      })?,
    )?;

    bot_table
  }};
}

macro_rules! create_player_table {
  ($lua_ctx: ident, $scope: ident, $net_ref: ident) => {{
    let player_table = $lua_ctx.create_table()?;

    player_table.set(
      "list_players",
      $scope.create_function(|_, area_id: String| {
        let mut net = $net_ref.borrow_mut();

        if let Some(area) = net.get_area(&area_id) {
          let connected_bots = area.get_connected_players();
          let result: Vec<String> = connected_bots.iter().map(|id| id.clone()).collect();

          Ok(result)
        } else {
          Err(create_area_error(&area_id))
        }
      })?,
    )?;

    player_table.set(
      "is_player",
      $scope.create_function(|_, id: String| {
        let net = $net_ref.borrow();

        if let Some(_) = net.get_player(&id) {
          Ok(true)
        } else {
          Ok(false)
        }
      })?,
    )?;

    player_table.set(
      "get_player_area",
      $scope.create_function(|_, id: String| {
        let net = $net_ref.borrow_mut();

        if let Some(player) = net.get_player(&id) {
          Ok(player.area_id.clone())
        } else {
          Err(create_player_error(&id))
        }
      })?,
    )?;

    player_table.set(
      "get_player_position",
      $scope.create_function(|lua_ctx, id: String| {
        let net = $net_ref.borrow();

        if let Some(player) = net.get_player(&id) {
          let table = lua_ctx.create_table()?;
          table.set("x", player.x)?;
          table.set("y", player.y)?;
          table.set("z", player.z)?;

          Ok(table)
        } else {
          Err(create_player_error(&id))
        }
      })?,
    )?;

    player_table.set(
      "get_player_avatar",
      $scope.create_function(|_, id: String| {
        let net = $net_ref.borrow_mut();

        if let Some(player) = net.get_player(&id) {
          Ok(player.avatar_id)
        } else {
          Err(create_player_error(&id))
        }
      })?,
    )?;

    player_table
  }};
}

impl LuaPluginInterface {
  pub fn new() -> LuaPluginInterface {
    let plugin_interface = LuaPluginInterface {
      scripts: HashMap::<std::path::PathBuf, Lua>::new(),
      tick_listeners: Vec::new(),
      player_join_listeners: Vec::new(),
      player_disconnect_listeners: Vec::new(),
      player_move_listeners: Vec::new(),
      player_avatar_change_listeners: Vec::new(),
      player_emote_listeners: Vec::new(),
    };

    plugin_interface
  }

  fn load_scripts(&mut self, net_ref: &mut Net) -> std::io::Result<()> {
    use std::fs::{read_dir, read_to_string};

    let net_ref = RefCell::new(net_ref);

    for wrapped_dir_entry in read_dir("./scripts")? {
      let dir_path = wrapped_dir_entry?.path();
      let script_paths = [&dir_path, &dir_path.join("main.lua")];

      for script_path in &script_paths {
        if let Ok(script) = read_to_string(script_path) {
          if let Err(err) = self.load_script(&net_ref, dir_path.clone(), script) {
            println!("Failed to load script \"{}\": {}", dir_path.display(), err)
          }

          break;
        }
      }
    }

    Ok(())
  }

  fn load_script(
    &mut self,
    net_ref: &RefCell<&mut Net>,
    script_dir: std::path::PathBuf,
    script: String,
  ) -> rlua::Result<()> {
    let lua_env = Lua::new();

    lua_env.context(|lua_ctx| {
      let globals = lua_ctx.globals();

      lua_ctx.scope(|scope| -> rlua::Result<()> {
        globals.set("Areas", create_area_table!(lua_ctx, scope, net_ref))?;
        globals.set("Bots", create_bot_table!(lua_ctx, scope, net_ref))?;
        globals.set("Players", create_player_table!(lua_ctx, scope, net_ref))?;

        lua_ctx.load(&script).exec()?;

        Ok(())
      })?;

      if let Ok(_) = globals.get::<_, rlua::Function>("tick") {
        self.tick_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_player_join") {
        self.player_join_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_player_disconnect") {
        self.player_disconnect_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_player_move") {
        self.player_move_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_player_avatar_change") {
        self.player_avatar_change_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_player_emote") {
        self.player_emote_listeners.push(script_dir.clone());
      }

      Ok(())
    })?;

    self.scripts.insert(script_dir, lua_env);

    Ok(())
  }
}

macro_rules! create_event_handler {
  ($self: ident, $net: ident, $prefix: expr, $event: expr, $($args: expr), *) => {{
    let call_lua = || -> rlua::Result<()> {
      let net_ref = RefCell::new($net);

      // loop over scripts
      for script_dir in &paste! { $self.[<$event _listeners>] } {
        // grab the lua_env (should always be true)
        if let Some(lua_env) = $self.scripts.get_mut(script_dir) {
          // enter the lua context

          lua_env.context(|lua_ctx: rlua::Context |-> rlua::Result<()> {
            lua_ctx.scope(|scope| -> rlua::Result<()> {
              let globals = lua_ctx.globals();

              globals.set("Areas", create_area_table!(lua_ctx, scope, net_ref))?;
              globals.set("Bots", create_bot_table!(lua_ctx, scope, net_ref))?;
              globals.set("Players", create_player_table!(lua_ctx, scope, net_ref))?;

              let fn_name = concat!($prefix, $event);

              if let Ok(func) = globals.get::<_, rlua::Function>(fn_name) {
                if let Err(err) = func.call::<_, ()>(($($args, )*)) {
                  println!("Error in \"{}\", {}", script_dir.display(), err);
                }
              }
              Ok(())
            })?;

            Ok(())
          })?
        }
      }
      Ok(())
    };

    if let Err(err) = call_lua() {
      println!("{:#}", err);
    }
  }};
}

impl PluginInterface for LuaPluginInterface {
  fn init(&mut self, net: &mut Net) {
    if let Err(err) = self.load_scripts(net) {
      println!("Failed to load lua scripts: {}", err);
    }
  }

  fn tick(&mut self, net: &mut Net, delta_time: f64) {
    create_event_handler!(self, net, "", "tick", delta_time);
  }

  fn handle_player_join(&mut self, net: &mut Net, player_id: &String) {
    create_event_handler!(self, net, "handle_", "player_join", player_id.clone());
  }

  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &String) {
    create_event_handler!(self, net, "handle_", "player_disconnect", player_id.clone());
  }

  fn handle_player_move(&mut self, net: &mut Net, player_id: &String, x: f64, y: f64, z: f64) {
    create_event_handler!(
      self,
      net,
      "handle_",
      "player_move",
      player_id.clone(),
      x,
      y,
      z
    );
  }

  fn handle_player_avatar_change(&mut self, net: &mut Net, player_id: &String, avatar_id: u16) {
    create_event_handler!(
      self,
      net,
      "handle_",
      "player_avatar_change",
      player_id.clone(),
      avatar_id
    );
  }

  fn handle_player_emote(&mut self, net: &mut Net, player_id: &String, emote_id: u8) {
    create_event_handler!(
      self,
      net,
      "handle_",
      "player_emote",
      player_id.clone(),
      emote_id
    );
  }
}
