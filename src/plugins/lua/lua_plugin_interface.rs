use super::api::add_net_api;
use crate::net::Net;
use crate::plugins::PluginInterface;
use paste::paste;
use rlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct LuaPluginInterface {
  scripts: HashMap<std::path::PathBuf, Lua>,
  tick_listeners: Vec<std::path::PathBuf>,
  player_connect_listeners: Vec<std::path::PathBuf>,
  player_disconnect_listeners: Vec<std::path::PathBuf>,
  player_move_listeners: Vec<std::path::PathBuf>,
  player_avatar_change_listeners: Vec<std::path::PathBuf>,
  player_emote_listeners: Vec<std::path::PathBuf>,
}

impl LuaPluginInterface {
  pub fn new() -> LuaPluginInterface {
    let plugin_interface = LuaPluginInterface {
      scripts: HashMap::<std::path::PathBuf, Lua>::new(),
      tick_listeners: Vec::new(),
      player_connect_listeners: Vec::new(),
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
        let api_table = lua_ctx.create_table()?;
        add_net_api(&api_table, &scope, &net_ref)?;
        globals.set("Net", api_table)?;

        lua_ctx.load(&script).exec()?;

        Ok(())
      })?;

      if let Ok(_) = globals.get::<_, rlua::Function>("tick") {
        self.tick_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_player_connect") {
        self.player_connect_listeners.push(script_dir.clone());
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

              let api_table = lua_ctx.create_table()?;
              add_net_api(&api_table, &scope, &net_ref)?;
              globals.set("Net", api_table)?;


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

  fn tick(&mut self, net: &mut Net, delta_time: f32) {
    create_event_handler!(self, net, "", "tick", delta_time);
  }

  fn handle_player_connect(&mut self, net: &mut Net, player_id: &String) {
    create_event_handler!(self, net, "handle_", "player_connect", player_id.clone());
  }

  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &String) {
    create_event_handler!(self, net, "handle_", "player_disconnect", player_id.clone());
  }

  fn handle_player_move(&mut self, net: &mut Net, player_id: &String, x: f32, y: f32, z: f32) {
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
