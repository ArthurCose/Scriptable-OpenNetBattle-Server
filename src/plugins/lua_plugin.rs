use super::plugin::Plugin;
use crate::map::Map;
use paste::paste;
use rlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct LuaPlugin {
  scripts: HashMap<std::path::PathBuf, Lua>,
  tick_listeners: Vec<std::path::PathBuf>,
  player_join_listeners: Vec<std::path::PathBuf>,
  player_disconnect_listeners: Vec<std::path::PathBuf>,
  player_move_listeners: Vec<std::path::PathBuf>,
  player_avatar_change_listeners: Vec<std::path::PathBuf>,
  player_emote_listeners: Vec<std::path::PathBuf>,
}

impl LuaPlugin {
  pub fn new() -> LuaPlugin {
    let mut plugin = LuaPlugin {
      scripts: HashMap::<std::path::PathBuf, Lua>::new(),
      tick_listeners: Vec::new(),
      player_join_listeners: Vec::new(),
      player_disconnect_listeners: Vec::new(),
      player_move_listeners: Vec::new(),
      player_avatar_change_listeners: Vec::new(),
      player_emote_listeners: Vec::new(),
    };

    plugin.init();

    plugin
  }

  fn init(&mut self) {
    if let Err(err) = self.load_scripts() {
      println!("Failed to load lua scripts: {}", err);
    }
  }

  fn load_scripts(&mut self) -> std::io::Result<()> {
    use std::fs::{read_dir, read_to_string};

    for wrapped_dir_entry in read_dir("./lua")? {
      let dir_path = wrapped_dir_entry?.path();
      let script_path = dir_path.join("main.lua");

      if let Ok(script) = read_to_string(script_path) {
        if let Err(err) = self.load_script(dir_path.clone(), script) {
          println!("Failed to load script \"{}\": {}", dir_path.display(), err)
        }
      }
    }

    Ok(())
  }

  fn load_script(&mut self, script_dir: std::path::PathBuf, script: String) -> rlua::Result<()> {
    let lua_env = Lua::new();

    lua_env.context(|lua_ctx| {
      lua_ctx.load(&script).exec()?;

      let globals = lua_ctx.globals();

      globals.set("Map", lua_ctx.create_table()?)?;

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
  ($self: ident, $map_ref: ident, $prefix: expr, $event: expr, $($args: expr), *) => {{
    let mut call_lua = || -> rlua::Result<()> {
      // loop over scripts
      for script_dir in &paste! { $self.[<$event _listeners>] } {
        // grab the lua_env (should always be true)
        if let Some(lua_env) = $self.scripts.get_mut(script_dir) {
          // enter the lua context
          lua_env.context(|lua_ctx|-> rlua::Result<()> {
            let globals = lua_ctx.globals();
            let fn_name = concat!($prefix, $event);

            lua_ctx.scope(|scope| -> rlua::Result<()> {
              let get_tile = scope.create_function(|_, (x, y) : (usize, usize)| {
                let map = $map_ref.borrow();
                Ok(map.get_tile(x, y))
              })?;
              globals.set("Map.get_tile", get_tile)?;

              let set_tile = scope.create_function_mut(|_, (x, y, id) : (usize, usize, String)| {
                let mut map = $map_ref.borrow_mut();
                Ok(map.set_tile(x, y, id))
              })?;
              globals.set("set_tile", set_tile)?;

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

impl Plugin for LuaPlugin {
  fn tick(&mut self, map: &mut RefCell<Map>, delta_time: f64) {
    create_event_handler!(self, map, "", "tick", delta_time);
  }

  fn handle_player_join(&mut self, map: &mut RefCell<Map>, ticket: String) {
    create_event_handler!(self, map, "handle_", "player_join", ticket.clone());
  }

  fn handle_player_disconnect(&mut self, map: &mut RefCell<Map>, ticket: String) {
    create_event_handler!(self, map, "handle_", "player_disconnect", ticket.clone());
  }

  fn handle_player_move(&mut self, map: &mut RefCell<Map>, ticket: String, x: f64, y: f64, z: f64) {
    create_event_handler!(self, map, "handle_", "player_move", ticket.clone(), x, y, z);
  }

  fn handle_player_avatar_change(
    &mut self,
    map: &mut RefCell<Map>,
    ticket: String,
    avatar_id: u16,
  ) {
    create_event_handler!(
      self,
      map,
      "handle_",
      "player_avatar_change",
      ticket.clone(),
      avatar_id
    );
  }

  fn handle_player_emote(&mut self, map: &mut RefCell<Map>, ticket: String, emote_id: u8) {
    create_event_handler!(
      self,
      map,
      "handle_",
      "player_emote",
      ticket.clone(),
      emote_id
    );
  }
}
