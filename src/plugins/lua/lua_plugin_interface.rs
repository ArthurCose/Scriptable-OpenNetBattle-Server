use super::api::{ApiContext, LuaApi};
use crate::jobs::JobPromiseManager;
use crate::net::{BattleStats, Net, WidgetTracker};
use crate::plugins::PluginInterface;
use log::*;
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;

pub struct LuaPluginInterface {
  scripts: Vec<Lua>,
  all_scripts: Vec<usize>,
  widget_trackers: HashMap<String, WidgetTracker<usize>>,
  battle_trackers: HashMap<String, VecDeque<usize>>,
  promise_manager: JobPromiseManager,
  lua_api: LuaApi,
}

impl LuaPluginInterface {
  pub fn new() -> LuaPluginInterface {
    LuaPluginInterface {
      scripts: Vec::new(),
      all_scripts: Vec::new(),
      widget_trackers: HashMap::new(),
      battle_trackers: HashMap::new(),
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
        error!("{}", err)
      }
    }

    Ok(())
  }

  fn load_script(
    &mut self,
    net_ref: &mut Net,
    script_path: std::path::PathBuf,
  ) -> mlua::Result<()> {
    let net_ref = RefCell::new(net_ref);

    let script_index = self.scripts.len();
    self.scripts.push(Lua::new());
    self.all_scripts.push(script_index);

    let lua_ctx = self.scripts.last_mut().unwrap();

    let widget_tracker_ref = RefCell::new(&mut self.widget_trackers);
    let battle_tracker_ref = RefCell::new(&mut self.battle_trackers);
    let promise_manager_ref = RefCell::new(&mut self.promise_manager);

    let api_ctx = ApiContext {
      script_index,
      net_ref: &net_ref,
      widget_tracker_ref: &widget_tracker_ref,
      battle_tracker_ref: &battle_tracker_ref,
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

      // using require to load the script for better error messages (logs the path of the file)
      let require: mlua::Function = globals.get("require")?;
      require.call::<&str, ()>(final_path)?;

      Ok(())
    })?;

    lua_ctx
      .load(include_str!("api/deprecated_callbacks.lua"))
      .set_name("internal: deprecated_callbacks.lua")?
      .exec()?;

    Ok(())
  }
}

impl PluginInterface for LuaPluginInterface {
  fn init(&mut self, net: &mut Net) {
    if let Err(err) = self.load_scripts(net) {
      error!("Failed to load lua scripts: {}", err);
    }
  }

  fn tick(&mut self, net: &mut Net, delta_time: f32) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("delta_time", delta_time)?;

        callback.call(("tick", event))
      },
    );
  }

  fn handle_authorization(
    &mut self,
    net: &mut Net,
    identity: &str,
    host: &str,
    port: u16,
    data: &[u8],
  ) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let data_string = lua_ctx.create_string(data)?;

        let event = lua_ctx.create_table()?;
        event.set("identity", identity)?;
        event.set("host", host)?;
        event.set("port", port)?;
        event.set("data", data_string)?;

        callback.call(("authorization", event))
      },
    );
  }

  fn handle_player_request(&mut self, net: &mut Net, player_id: &str, data: &str) {
    self
      .widget_trackers
      .insert(player_id.to_string(), WidgetTracker::new());

    self
      .battle_trackers
      .insert(player_id.to_string(), VecDeque::new());

    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("data", data)?;

        callback.call(("player_request", event))
      },
    );
  }

  fn handle_player_connect(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("player_connect", event))
      },
    );
  }

  fn handle_player_join(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("player_join", event))
      },
    );
  }

  fn handle_player_transfer(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("player_area_transfer", event))
      },
    );
  }

  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &str) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("player_disconnect", event))
      },
    );

    self.widget_trackers.remove(player_id);
    self.battle_trackers.remove(player_id);
  }

  fn handle_player_move(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("x", x)?;
        event.set("y", y)?;
        event.set("z", z)?;

        callback.call(("player_move", event))
      },
    );
  }

  fn handle_player_avatar_change(
    &mut self,
    net: &mut Net,
    player_id: &str,
    texture_path: &str,
    animation_path: &str,
    name: &str,
    element: &str,
    max_health: u32,
  ) -> bool {
    use std::cell::Cell;
    use std::rc::Rc;

    let prevent_default = Rc::new(Cell::new(false));

    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let prevent_default_reference = prevent_default.clone();

        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("texture_path", texture_path)?;
        event.set("animation_path", animation_path)?;
        event.set("name", name)?;
        event.set("element", element)?;
        event.set("max_health", max_health)?;
        event.set(
          "prevent_default",
          lua_ctx.create_function(move |_, _: ()| {
            prevent_default_reference.clone().set(true);
            Ok(())
          })?,
        )?;

        callback.call(("player_avatar_change", event))
      },
    );

    prevent_default.get()
  }

  fn handle_player_emote(&mut self, net: &mut Net, player_id: &str, emote_id: u8) -> bool {
    use std::cell::Cell;
    use std::rc::Rc;

    let prevent_default = Rc::new(Cell::new(false));

    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let prevent_default_reference = prevent_default.clone();

        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("emote", emote_id)?;
        event.set(
          "prevent_default",
          lua_ctx.create_function(move |_, _: ()| {
            prevent_default_reference.clone().set(true);
            Ok(())
          })?,
        )?;

        callback.call(("player_emote", event))
      },
    );

    prevent_default.get()
  }

  fn handle_custom_warp(&mut self, net: &mut Net, player_id: &str, tile_object_id: u32) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("object_id", tile_object_id)?;

        callback.call(("custom_warp", event))
      },
    );
  }

  fn handle_object_interaction(
    &mut self,
    net: &mut Net,
    player_id: &str,
    tile_object_id: u32,
    button: u8,
  ) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("object_id", tile_object_id)?;
        event.set("button", button)?;

        callback.call(("object_interaction", event))
      },
    );
  }

  fn handle_actor_interaction(
    &mut self,
    net: &mut Net,
    player_id: &str,
    actor_id: &str,
    button: u8,
  ) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("actor_id", actor_id)?;
        event.set("button", button)?;

        callback.call(("actor_interaction", event))
      },
    );
  }

  fn handle_tile_interaction(
    &mut self,
    net: &mut Net,
    player_id: &str,
    x: f32,
    y: f32,
    z: f32,
    button: u8,
  ) {
    handle_event(
      &mut self.scripts,
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("x", x)?;
        event.set("y", y)?;
        event.set("z", z)?;
        event.set("button", button)?;

        callback.call(("tile_interaction", event))
      },
    );
  }

  fn handle_textbox_response(&mut self, net: &mut Net, player_id: &str, response: u8) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.pop_textbox() {
      script_index
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("response", response)?;

        callback.call(("textbox_response", event))
      },
    );
  }

  fn handle_prompt_response(&mut self, net: &mut Net, player_id: &str, response: String) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.pop_textbox() {
      script_index
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("response", response.as_str())?;

        callback.call(("textbox_response", event))
      },
    );
  }

  fn handle_board_open(&mut self, net: &mut Net, player_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    tracker.open_board();

    let script_index = if let Some(script_index) = tracker.current_board() {
      script_index.clone()
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("board_open", event))
      },
    );
  }

  fn handle_board_close(&mut self, net: &mut Net, player_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.close_board() {
      script_index
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("board_close", event))
      },
    );
  }

  fn handle_post_request(&mut self, net: &mut Net, player_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.current_board() {
      script_index.clone()
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("post_request", event))
      },
    );
  }

  fn handle_post_selection(&mut self, net: &mut Net, player_id: &str, post_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.current_board() {
      script_index.clone()
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("post_id", post_id)?;

        callback.call(("post_selection", event))
      },
    );
  }

  fn handle_shop_close(&mut self, net: &mut Net, player_id: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.close_shop() {
      script_index
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;

        callback.call(("shop_close", event))
      },
    );
  }

  fn handle_shop_purchase(&mut self, net: &mut Net, player_id: &str, item_name: &str) {
    let tracker = self.widget_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.current_shop() {
      script_index.clone()
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("item_name", item_name)?;

        callback.call(("shop_purchase", event))
      },
    );
  }

  fn handle_battle_results(&mut self, net: &mut Net, player_id: &str, battle_stats: &BattleStats) {
    let tracker = self.battle_trackers.get_mut(player_id).unwrap();

    let script_index = if let Some(script_index) = tracker.pop_front() {
      script_index
    } else {
      // protect against attackers
      return;
    };

    handle_event(
      &mut self.scripts,
      &[script_index],
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("player_id", player_id)?;
        event.set("health", battle_stats.health)?;
        event.set("score", battle_stats.score)?;
        event.set("time", battle_stats.time)?;
        event.set("ran", battle_stats.ran)?;
        event.set("emotion", battle_stats.emotion)?;
        event.set("turns", battle_stats.turns)?;

        let mut enemy_tables = Vec::new();
        enemy_tables.reserve(battle_stats.enemies.len());

        for enemy in &battle_stats.enemies {
          let enemy_table = lua_ctx.create_table()?;
          enemy_table.set("id", enemy.id.as_str())?;
          enemy_table.set("health", enemy.health)?;
          enemy_tables.push(enemy_table);
        }

        event.set("enemies", enemy_tables)?;

        callback.call(("battle_results", event))
      },
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
      &self.all_scripts,
      &mut self.widget_trackers,
      &mut self.battle_trackers,
      &mut self.promise_manager,
      &mut self.lua_api,
      net,
      |lua_ctx, callback| {
        let event = lua_ctx.create_table()?;
        event.set("host", socket_address.ip().to_string())?;
        event.set("port", socket_address.port())?;
        event.set("data", lua_ctx.create_string(data)?)?;

        callback.call(("server_message", event))
      },
    );
  }
}

#[allow(clippy::too_many_arguments)]
fn handle_event<F>(
  scripts: &mut Vec<Lua>,
  event_listeners: &[usize],
  widget_tracker: &mut HashMap<String, WidgetTracker<usize>>,
  battle_tracker: &mut HashMap<String, VecDeque<usize>>,
  promise_manager: &mut JobPromiseManager,
  lua_api: &mut LuaApi,
  net: &mut Net,
  fn_caller: F,
) where
  F: for<'lua> FnMut(&'lua mlua::Lua, mlua::Function<'lua>) -> mlua::Result<()>,
{
  let mut fn_caller = fn_caller;

  let call_lua = || -> mlua::Result<()> {
    let net_ref = RefCell::new(net);
    let widget_tracker_ref = RefCell::new(widget_tracker);
    let battle_tracker_ref = RefCell::new(battle_tracker);
    let promise_manager_ref = RefCell::new(promise_manager);

    // loop over scripts
    for script_index in event_listeners {
      let lua_ctx = scripts.get_mut(*script_index).unwrap();

      let api_ctx = ApiContext {
        script_index: *script_index,
        net_ref: &net_ref,
        widget_tracker_ref: &widget_tracker_ref,
        battle_tracker_ref: &battle_tracker_ref,
        promise_manager_ref: &promise_manager_ref,
      };

      lua_api.inject_dynamic(lua_ctx, api_ctx, |lua_ctx| {
        let globals = lua_ctx.globals();
        let net_table: mlua::Table = globals.get("Net")?;

        if let Ok(func) = net_table.get::<_, mlua::Function>("emit") {
          let binded_func = func.bind(net_table)?;

          if let Err(err) = fn_caller(lua_ctx, binded_func) {
            error!("{}", err);
          }
        }

        Ok(())
      })?;
    }
    Ok(())
  };

  if let Err(err) = call_lua() {
    error!("{:#}", err);
  }
}
