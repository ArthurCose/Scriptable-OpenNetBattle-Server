use super::api::add_net_api;
use crate::net::Net;
use crate::plugins::PluginInterface;
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
  object_interaction_listeners: Vec<std::path::PathBuf>,
  navi_interaction_listeners: Vec<std::path::PathBuf>,
  tile_interaction_listeners: Vec<std::path::PathBuf>,
}

impl LuaPluginInterface {
  pub fn new() -> LuaPluginInterface {
    let plugin_interface = LuaPluginInterface {
      scripts: HashMap::new(),
      tick_listeners: Vec::new(),
      player_connect_listeners: Vec::new(),
      player_disconnect_listeners: Vec::new(),
      player_move_listeners: Vec::new(),
      player_avatar_change_listeners: Vec::new(),
      player_emote_listeners: Vec::new(),
      object_interaction_listeners: Vec::new(),
      navi_interaction_listeners: Vec::new(),
      tile_interaction_listeners: Vec::new(),
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

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_object_interaction") {
        self.object_interaction_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_navi_interaction") {
        self.navi_interaction_listeners.push(script_dir.clone());
      }

      if let Ok(_) = globals.get::<_, rlua::Function>("handle_tile_interaction") {
        self.tile_interaction_listeners.push(script_dir.clone());
      }

      Ok(())
    })?;

    self.scripts.insert(script_dir, lua_env);

    Ok(())
  }
}

impl PluginInterface for LuaPluginInterface {
  fn init(&mut self, net: &mut Net) {
    if let Err(err) = self.load_scripts(net) {
      println!("Failed to load lua scripts: {}", err);
    }
  }

  fn tick(&mut self, net: &mut Net, delta_time: f32) {
    handle_event(
      &mut self.scripts,
      &self.tick_listeners,
      net,
      "tick",
      |callback| callback.call(delta_time),
    );
  }

  fn handle_player_connect(&mut self, net: &mut Net, player_id: &String) {
    handle_event(
      &mut self.scripts,
      &self.player_connect_listeners,
      net,
      "handle_player_connect",
      |callback| callback.call(player_id.clone()),
    );
  }

  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &String) {
    handle_event(
      &mut self.scripts,
      &self.player_disconnect_listeners,
      net,
      "handle_player_disconnect",
      |callback| callback.call(player_id.clone()),
    );
  }

  fn handle_player_move(&mut self, net: &mut Net, player_id: &String, x: f32, y: f32, z: f32) {
    handle_event(
      &mut self.scripts,
      &self.player_move_listeners,
      net,
      "handle_player_move",
      |callback| callback.call((player_id.clone(), x, y, z)),
    );
  }

  fn handle_player_avatar_change(
    &mut self,
    net: &mut Net,
    player_id: &String,
    texture_path: &String,
    animation_path: &String,
  ) {
    handle_event(
      &mut self.scripts,
      &self.player_avatar_change_listeners,
      net,
      "handle_player_avatar_change",
      |callback| {
        callback.call((
          player_id.clone(),
          texture_path.clone(),
          animation_path.clone(),
        ))
      },
    );
  }

  fn handle_player_emote(&mut self, net: &mut Net, player_id: &String, emote_id: u8) {
    handle_event(
      &mut self.scripts,
      &self.player_emote_listeners,
      net,
      "handle_player_emote",
      |callback| callback.call((player_id.clone(), emote_id)),
    );
  }

  fn handle_object_interaction(&mut self, net: &mut Net, player_id: &String, tile_object_id: u32) {
    handle_event(
      &mut self.scripts,
      &self.object_interaction_listeners,
      net,
      "handle_object_interaction",
      |callback| callback.call((player_id.clone(), tile_object_id)),
    );
  }

  fn handle_navi_interaction(&mut self, net: &mut Net, player_id: &String, navi_id: &String) {
    handle_event(
      &mut self.scripts,
      &self.navi_interaction_listeners,
      net,
      "handle_navi_interaction",
      |callback| callback.call((player_id.clone(), navi_id.clone())),
    );
  }

  fn handle_tile_interaction(&mut self, net: &mut Net, player_id: &String, x: f32, y: f32, z: f32) {
    handle_event(
      &mut self.scripts,
      &self.tile_interaction_listeners,
      net,
      "handle_tile_interaction",
      |callback| callback.call((player_id.clone(), x, y, z)),
    );
  }
}

fn handle_event<F>(
  scripts: &mut HashMap<std::path::PathBuf, Lua>,
  event_listeners: &Vec<std::path::PathBuf>,
  net: &mut Net,
  event_fn_name: &str,
  fn_caller: F,
) where
  F: for<'lua> Fn(rlua::Function<'lua>) -> rlua::Result<()>,
{
  let call_lua = || -> rlua::Result<()> {
    let net_ref = RefCell::new(net);

    // loop over scripts
    for script_dir in event_listeners {
      // grab the lua_env (should always be true)
      if let Some(lua_env) = scripts.get_mut(script_dir) {
        // enter the lua context

        lua_env.context(|lua_ctx: rlua::Context| -> rlua::Result<()> {
          lua_ctx.scope(|scope| -> rlua::Result<()> {
            let globals = lua_ctx.globals();

            let api_table = lua_ctx.create_table()?;
            add_net_api(&api_table, &scope, &net_ref)?;
            globals.set("Net", api_table)?;

            if let Ok(func) = globals.get::<_, rlua::Function>(event_fn_name) {
              if let Err(err) = fn_caller(func) {
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
}
