use super::plugin_interface::PluginInterface;
use crate::area::{Area, Bot};
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

// abusing macro to auto resolve lifetime
macro_rules! create_map_table {
  ($lua_ctx: ident, $scope: ident, $area_ref: ident) => {{
    let map_table = $lua_ctx.create_table()?;

    map_table.set(
      "get_width",
      $scope.create_function(|_, ()| {
        let mut area = $area_ref.borrow_mut();
        Ok(area.get_map().get_width())
      })?,
    )?;

    map_table.set(
      "get_height",
      $scope.create_function(|_, ()| {
        let mut area = $area_ref.borrow_mut();
        Ok(area.get_map().get_height())
      })?,
    )?;

    map_table.set(
      "get_tile",
      $scope.create_function(|_, (x, y): (usize, usize)| {
        let mut area = $area_ref.borrow_mut();
        Ok(area.get_map().get_tile(x, y))
      })?,
    )?;

    map_table.set(
      "set_tile",
      $scope.create_function(|_, (x, y, id): (usize, usize, String)| {
        let mut area = $area_ref.borrow_mut();
        Ok(area.get_map().set_tile(x, y, id))
      })?,
    )?;

    map_table
  }};
}

macro_rules! create_bot_table {
  ($lua_ctx: ident, $scope: ident, $area_ref: ident) => {{
    let bot_table = $lua_ctx.create_table()?;

    bot_table.set(
      "create_bot",
      $scope.create_function(
        |_, (id, avatar_id, x, y, z): (String, u16, f64, f64, f64)| {
          let mut area = $area_ref.borrow_mut();

          let bot = Bot {
            id,
            avatar_id,
            x,
            y,
            z,
          };

          area.add_bot(bot);

          Ok(())
        },
      )?,
    )?;

    bot_table.set(
      "remove_bot",
      $scope.create_function(|_, id: String| {
        let mut area = $area_ref.borrow_mut();

        area.remove_bot(&id);

        Ok(())
      })?,
    )?;

    bot_table.set(
      "get_bot_position",
      $scope.create_function(|_, id: String| {
        let area = $area_ref.borrow();

        if let Some(bot) = area.get_bot(&id) {
          Ok(vec![bot.x, bot.y, bot.z])
        } else {
          Err(rlua::Error::RuntimeError(String::from(
            "No bot matching that ID found.",
          )))
        }
      })?,
    )?;

    bot_table.set(
      "move_bot",
      $scope.create_function(|_, (id, x, y, z): (String, f64, f64, f64)| {
        let mut area = $area_ref.borrow_mut();

        area.move_bot(&id, x, y, z);

        Ok(())
      })?,
    )?;

    bot_table.set(
      "set_bot_avatar",
      $scope.create_function(|_, (id, avatar_id): (String, u16)| {
        let mut area = $area_ref.borrow_mut();

        if let None = area.get_bot(&id) {
          return Err(rlua::Error::RuntimeError(String::from(
            "No bot matching that ID found.",
          )));
        }

        area.set_bot_avatar(&id, avatar_id);

        Ok(())
      })?,
    )?;

    bot_table.set(
      "set_bot_emote",
      $scope.create_function(|_, (id, emote_id): (String, u8)| {
        let mut area = $area_ref.borrow_mut();

        if let None = area.get_bot(&id) {
          return Err(rlua::Error::RuntimeError(String::from(
            "No bot matching that ID found.",
          )));
        }

        area.set_bot_emote(&id, emote_id);

        Ok(())
      })?,
    )?;

    bot_table
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

  fn load_scripts(&mut self, area_ref: &mut Area) -> std::io::Result<()> {
    use std::fs::{read_dir, read_to_string};

    let area_ref = RefCell::new(area_ref);

    for wrapped_dir_entry in read_dir("./scripts")? {
      let dir_path = wrapped_dir_entry?.path();
      let script_path = dir_path.join("main.lua");

      if let Ok(script) = read_to_string(script_path) {
        if let Err(err) = self.load_script(&area_ref, dir_path.clone(), script) {
          println!("Failed to load script \"{}\": {}", dir_path.display(), err)
        }
      }
    }

    Ok(())
  }

  fn load_script(
    &mut self,
    area_ref: &RefCell<&mut Area>,
    script_dir: std::path::PathBuf,
    script: String,
  ) -> rlua::Result<()> {
    let lua_env = Lua::new();

    lua_env.context(|lua_ctx| {
      let globals = lua_ctx.globals();

      lua_ctx.scope(|scope| -> rlua::Result<()> {
        globals.set("Map", create_map_table!(lua_ctx, scope, area_ref))?;
        globals.set("Bots", create_bot_table!(lua_ctx, scope, area_ref))?;

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
  ($self: ident, $area: ident, $prefix: expr, $event: expr, $($args: expr), *) => {{
    let call_lua = || -> rlua::Result<()> {
      let area_ref = RefCell::new($area);

      // loop over scripts
      for script_dir in &paste! { $self.[<$event _listeners>] } {
        // grab the lua_env (should always be true)
        if let Some(lua_env) = $self.scripts.get_mut(script_dir) {
          // enter the lua context

          lua_env.context(|lua_ctx: rlua::Context |-> rlua::Result<()> {
            lua_ctx.scope(|scope| -> rlua::Result<()> {
              let globals = lua_ctx.globals();

              globals.set("Map", create_map_table!(lua_ctx, scope, area_ref))?;
              globals.set("Bots", create_bot_table!(lua_ctx, scope, area_ref))?;

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
  fn init(&mut self, area: &mut Area) {
    if let Err(err) = self.load_scripts(area) {
      println!("Failed to load lua scripts: {}", err);
    }
  }

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
