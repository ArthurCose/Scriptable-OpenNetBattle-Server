use super::api::{ApiContext, LuaApi};
use crate::jobs::JobPromiseManager;
use crate::net::{Net, WidgetTracker};
use crate::plugins::PluginInterface;
use rlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct LuaPluginInterface {
  scripts: HashMap<std::path::PathBuf, Lua>,
  tick_listeners: Vec<std::path::PathBuf>,
  player_request_listeners: Vec<std::path::PathBuf>,
  player_connect_listeners: Vec<std::path::PathBuf>,
  player_join_listeners: Vec<std::path::PathBuf>,
  player_transfer_listeners: Vec<std::path::PathBuf>,
  player_disconnect_listeners: Vec<std::path::PathBuf>,
  player_move_listeners: Vec<std::path::PathBuf>,
  player_avatar_change_listeners: Vec<std::path::PathBuf>,
  player_emote_listeners: Vec<std::path::PathBuf>,
  custom_warp_listeners: Vec<std::path::PathBuf>,
  object_interaction_listeners: Vec<std::path::PathBuf>,
  actor_interaction_listeners: Vec<std::path::PathBuf>,
  tile_interaction_listeners: Vec<std::path::PathBuf>,
  server_message_listeners: Vec<std::path::PathBuf>,
  widget_trackers: HashMap<String, WidgetTracker<std::path::PathBuf>>,
  promise_manager: JobPromiseManager,
  lua_api: LuaApi,
}

impl LuaPluginInterface {
  pub fn new() -> LuaPluginInterface {
    LuaPluginInterface {
      scripts: HashMap::new(),
      tick_listeners: Vec::new(),
      player_request_listeners: Vec::new(),
      player_connect_listeners: Vec::new(),
      player_join_listeners: Vec::new(),
      player_transfer_listeners: Vec::new(),
      player_disconnect_listeners: Vec::new(),
      player_move_listeners: Vec::new(),
      player_avatar_change_listeners: Vec::new(),
      player_emote_listeners: Vec::new(),
      custom_warp_listeners: Vec::new(),
      object_interaction_listeners: Vec::new(),
      actor_interaction_listeners: Vec::new(),
      tile_interaction_listeners: Vec::new(),
      server_message_listeners: Vec::new(),
      widget_trackers: HashMap::new(),
      promise_manager: JobPromiseManager::new(),
      lua_api: LuaApi::new(),
    }
  }

  fn load_scripts(&mut self, net_ref: &mut Net) -> std::io::Result<()> {
    use std::fs::read_dir;

    for wrapped_dir_entry in read_dir("./scripts")? {
      let dir_path = wrapped_dir_entry?.path();
      let mut script_path = dir_path;

      let extension = script_path.extension().unwrap_or_default().to_str();

      if !matches!(extension, Some("lua")) {
        script_path = script_path.join("main.lua")
      }

      if !script_path.exists() {
        continue;
      }

      if let Err(err) = self.load_script(net_ref, script_path.to_path_buf()) {
        println!("{}", err)
      }
    }

    Ok(())
  }

  fn load_script(
    &mut self,
    net_ref: &mut Net,
    script_path: std::path::PathBuf,
  ) -> rlua::Result<()> {
    let net_ref = RefCell::new(net_ref);

    let lua_env = unsafe { Lua::new_with_debug() };

    lua_env.context(|lua_ctx| {
      let widget_tracker_ref = RefCell::new(&mut self.widget_trackers);
      let promise_manager_ref = RefCell::new(&mut self.promise_manager);

      let api_ctx = ApiContext {
        script_path: &script_path,
        net_ref: &net_ref,
        widget_tracker_ref: &widget_tracker_ref,
        promise_manager_ref: &promise_manager_ref,
      };

      let globals = lua_ctx.globals();

      self.lua_api.inject_static(&lua_ctx)?;

      self.lua_api.inject_dynamic(lua_ctx, api_ctx, |_| {
        let parent_path = script_path
          .parent()
          .unwrap_or_else(|| std::path::Path::new(""));
        let stem = script_path.file_stem().unwrap_or_default();
        let path = parent_path.join(stem);
        let path_str = path.to_str().unwrap_or_default();

        let final_path = &path_str[2..]; // chop off the ./

        let require: rlua::Function = globals.get("require")?;
        require.call::<&str, ()>(final_path)?;

        Ok(())
      })?;

      self.tick_listeners.push(script_path.clone());

      if globals
        .get::<_, rlua::Function>("handle_player_request")
        .is_ok()
      {
        self.player_request_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_player_connect")
        .is_ok()
      {
        self.player_connect_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_player_join")
        .is_ok()
      {
        self.player_join_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_player_transfer")
        .is_ok()
      {
        self.player_transfer_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_player_disconnect")
        .is_ok()
      {
        self.player_disconnect_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_player_move")
        .is_ok()
      {
        self.player_move_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_player_avatar_change")
        .is_ok()
      {
        self
          .player_avatar_change_listeners
          .push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_player_emote")
        .is_ok()
      {
        self.player_emote_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_custom_warp")
        .is_ok()
      {
        self.custom_warp_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_object_interaction")
        .is_ok()
      {
        self.object_interaction_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_actor_interaction")
        .is_ok()
      {
        self.actor_interaction_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_tile_interaction")
        .is_ok()
      {
        self.tile_interaction_listeners.push(script_path.clone());
      }

      if globals
        .get::<_, rlua::Function>("handle_server_message")
        .is_ok()
      {
        self.server_message_listeners.push(script_path.clone());
      }

      Ok(())
    })?;

    self.scripts.insert(script_path, lua_env);

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
    // async_api.lua
    handle_event(
      &mut self.scripts,
      &self.tick_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "_server_internal_tick",
      |_, callback| callback.call(delta_time),
    );
  }

  fn handle_player_request(&mut self, net: &mut Net, player_id: &str, data: &str) {
    self
      .widget_trackers
      .insert(player_id.to_string(), WidgetTracker::new());

    handle_event(
      &mut self.scripts,
      &self.player_request_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_request",
      |_, callback| callback.call((player_id, data)),
    );
  }

  fn handle_player_connect(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.player_connect_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_connect",
      |_, callback| callback.call(player_id),
    );
  }

  fn handle_player_join(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.player_join_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_join",
      |_, callback| callback.call(player_id),
    );
  }

  fn handle_player_transfer(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.player_transfer_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_transfer",
      |_, callback| callback.call(player_id),
    );
  }

  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.player_disconnect_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_disconnect",
      |_, callback| callback.call(player_id),
    );

    self.widget_trackers.remove(player_id);
  }

  fn handle_player_move(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32) {
    handle_event(
      &mut self.scripts,
      &self.player_move_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_move",
      |_, callback| callback.call((player_id, x, y, z)),
    );
  }

  fn handle_player_avatar_change(
    &mut self,
    net: &mut Net,
    player_id: &str,
    texture_path: &str,
    animation_path: &str,
  ) -> bool {
    let mut prevent_default = false;

    handle_event(
      &mut self.scripts,
      &self.player_avatar_change_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_avatar_change",
      |_, callback| {
        let return_value: Option<bool> =
          callback.call((player_id, texture_path, animation_path))?;

        prevent_default |= return_value.unwrap_or_default();

        Ok(())
      },
    );

    prevent_default
  }

  fn handle_player_emote(&mut self, net: &mut Net, player_id: &str, emote_id: u8) -> bool {
    let mut prevent_default = false;

    handle_event(
      &mut self.scripts,
      &self.player_emote_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_player_emote",
      |_, callback| {
        let return_value: Option<bool> = callback.call((player_id, emote_id))?;

        prevent_default |= return_value.unwrap_or_default();

        Ok(())
      },
    );

    prevent_default
  }

  fn handle_custom_warp(&mut self, net: &mut Net, player_id: &str, tile_object_id: u32) {
    handle_event(
      &mut self.scripts,
      &self.custom_warp_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_custom_warp",
      |_, callback| callback.call((player_id, tile_object_id)),
    );
  }

  fn handle_object_interaction(&mut self, net: &mut Net, player_id: &str, tile_object_id: u32) {
    handle_event(
      &mut self.scripts,
      &self.object_interaction_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_object_interaction",
      |_, callback| callback.call((player_id, tile_object_id)),
    );
  }

  fn handle_actor_interaction(&mut self, net: &mut Net, player_id: &str, actor_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.actor_interaction_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_actor_interaction",
      |_, callback| callback.call((player_id, actor_id)),
    );
  }

  fn handle_tile_interaction(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32) {
    handle_event(
      &mut self.scripts,
      &self.tile_interaction_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_tile_interaction",
      |_, callback| callback.call((player_id, x, y, z)),
    );
  }

  fn handle_textbox_response(&mut self, net: &mut Net, player_id: &str, response: u8) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_path = if let Some(script_path) = tracker.pop_textbox() {
      script_path
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_path],
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_textbox_response",
      |_, callback| callback.call((player_id, response)),
    );
  }

  fn handle_board_open(&mut self, net: &mut Net, player_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    tracker.open_board();

    let script_path = if let Some(script_path) = tracker.current_board() {
      script_path.clone()
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_path],
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_board_open",
      |_, callback| callback.call(player_id),
    );
  }

  fn handle_board_close(&mut self, net: &mut Net, player_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_path = if let Some(script_path) = tracker.close_board() {
      script_path
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_path],
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_board_close",
      |_, callback| callback.call(player_id),
    );
  }

  fn handle_post_request(&mut self, net: &mut Net, player_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_path = if let Some(script_path) = tracker.current_board() {
      script_path.clone()
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_path],
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_post_request",
      |_, callback| callback.call(player_id),
    );
  }

  fn handle_post_selection(&mut self, net: &mut Net, player_id: &str, post_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_path = if let Some(script_path) = tracker.current_board() {
      script_path.clone()
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_path],
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_post_selection",
      |_, callback| callback.call((player_id, post_id)),
    );
  }

  fn handle_server_message(
    &mut self,
    net: &mut Net,
    socket_address: std::net::SocketAddr,
    data: &[u8],
  ) {
    handle_event(
      &mut self.scripts,
      &self.server_message_listeners,
      &mut self.widget_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      "handle_server_message",
      |lua_ctx, callback| {
        let lua_data_string = lua_ctx.create_string(data)?;
        let ip_string = socket_address.ip().to_string();
        let port = socket_address.port();

        callback.call((ip_string, port, lua_data_string))
      },
    );
  }
}

#[allow(clippy::too_many_arguments)]
fn handle_event<F>(
  scripts: &mut HashMap<std::path::PathBuf, Lua>,
  event_listeners: &[std::path::PathBuf],
  widget_tracker: &mut HashMap<String, WidgetTracker<std::path::PathBuf>>,
  promise_manager: &mut JobPromiseManager,
  lua_api: &mut LuaApi,
  net: &mut Net,
  event_fn_name: &str,
  fn_caller: F,
) where
  F: for<'lua> FnMut(rlua::Context<'lua>, rlua::Function<'lua>) -> rlua::Result<()>,
{
  let mut fn_caller = fn_caller;

  let call_lua = || -> rlua::Result<()> {
    let net_ref = RefCell::new(net);
    let widget_tracker_ref = RefCell::new(widget_tracker);
    let promise_manager_ref = RefCell::new(promise_manager);

    // loop over scripts
    for script_path in event_listeners {
      // grab the lua_env (should always be true)
      if let Some(lua_env) = scripts.get_mut(script_path) {
        // enter the lua context
        let api_ctx = ApiContext {
          script_path,
          net_ref: &net_ref,
          widget_tracker_ref: &widget_tracker_ref,
          promise_manager_ref: &promise_manager_ref,
        };

        lua_env.context(|lua_ctx| -> rlua::Result<()> {
          lua_api.inject_dynamic(lua_ctx, api_ctx, |lua_ctx| {
            let globals = lua_ctx.globals();

            if let Ok(func) = globals.get::<_, rlua::Function>(event_fn_name) {
              if let Err(err) = fn_caller(lua_ctx, func) {
                println!("{}", err);
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
