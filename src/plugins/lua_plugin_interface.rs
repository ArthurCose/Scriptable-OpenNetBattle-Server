use super::plugin_interface::PluginInterface;
use crate::area::Area;
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

impl LuaPluginInterface {
  pub fn new() -> LuaPluginInterface {
    let mut plugin_interface = LuaPluginInterface {
      scripts: HashMap::<std::path::PathBuf, Lua>::new(),
      tick_listeners: Vec::new(),
      player_join_listeners: Vec::new(),
      player_disconnect_listeners: Vec::new(),
      player_move_listeners: Vec::new(),
      player_avatar_change_listeners: Vec::new(),
      player_emote_listeners: Vec::new(),
    };

    plugin_interface.init();

    plugin_interface
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
  ($self: ident, $area: ident, $prefix: expr, $event: expr, $($args: expr), *) => {{
    let call_lua = || -> rlua::Result<()> {
      let area_ref = RefCell::new($area);

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
                let mut area = area_ref.borrow_mut();
                Ok(area.get_map().get_tile(x, y))
              })?;
              globals.set("Map.get_tile", get_tile)?;

              let set_tile = scope.create_function_mut(|_, (x, y, id) : (usize, usize, String)| {
                let mut area = area_ref.borrow_mut();
                Ok(area.get_map().set_tile(x, y, id))
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

impl PluginInterface for LuaPluginInterface {
  fn tick(&mut self, area: &mut Area, delta_time: f64) {
    create_event_handler!(self, area, "", "tick", delta_time);
  }

  fn handle_player_join(&mut self, area: &mut Area, player_id: &String) {
    create_event_handler!(self, area, "handle_", "player_join", player_id.clone());
  }

  fn handle_player_disconnect(&mut self, area: &mut Area, player_id: &String) {
    create_event_handler!(
      self,
      area,
      "handle_",
      "player_disconnect",
      player_id.clone()
    );
  }

  fn handle_player_move(&mut self, area: &mut Area, player_id: &String, x: f64, y: f64, z: f64) {
    create_event_handler!(
      self,
      area,
      "handle_",
      "player_move",
      player_id.clone(),
      x,
      y,
      z
    );
  }

  fn handle_player_avatar_change(&mut self, area: &mut Area, player_id: &String, avatar_id: u16) {
    create_event_handler!(
      self,
      area,
      "handle_",
      "player_avatar_change",
      player_id.clone(),
      avatar_id
    );
  }

  fn handle_player_emote(&mut self, area: &mut Area, player_id: &String, emote_id: u8) {
    create_event_handler!(
      self,
      area,
      "handle_",
      "player_emote",
      player_id.clone(),
      emote_id
    );
  }
}
