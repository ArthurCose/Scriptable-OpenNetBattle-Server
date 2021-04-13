use super::lua_errors::{create_area_error, create_bot_error};
use super::LuaAPI;
use crate::net::{Actor, Direction};

pub fn inject_dynamic(lua_api: &mut LuaAPI) {
  lua_api.add_dynamic_function("Net", "list_bots", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let connected_bots = area.get_connected_bots();
      let result: Vec<String> = connected_bots.to_vec();

      lua_ctx.pack_multi(result)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "create_bot", |api_ctx, lua_ctx, params| {
    let (id, table): (String, rlua::Table) = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();

    let name: Option<String> = table.get("name")?;
    let area_id: Option<String> = table.get("area_id")?;
    let texture_path: Option<String> = table.get("texture_path")?;
    let animation_path: Option<String> = table.get("animation_path")?;
    let x: Option<f32> = table.get("x")?;
    let y: Option<f32> = table.get("y")?;
    let z: Option<f32> = table.get("z")?;
    let direction: Option<String> = table.get("direction")?;
    let solid: Option<bool> = table.get("solid")?;

    let area_id = area_id.unwrap_or_else(|| String::from("default"));

    if let Some(area) = net.get_area(&area_id) {
      let map = area.get_map();
      let spawn = map.get_spawn();
      let spawn_direction = map.get_spawn_direction();

      let direction = direction
        .map(|string| Direction::from(&string))
        .unwrap_or(spawn_direction);

      let bot = Actor {
        id,
        name: name.unwrap_or_default(),
        area_id,
        texture_path: texture_path.unwrap_or_default(),
        animation_path: animation_path.unwrap_or_default(),
        mugshot_texture_path: String::default(),
        mugshot_animation_path: String::default(),
        direction,
        x: x.unwrap_or(spawn.0),
        y: y.unwrap_or(spawn.1),
        z: z.unwrap_or(spawn.2),
        solid: solid.unwrap_or_default(),
      };

      net.add_bot(bot);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "is_bot", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    lua_ctx.pack_multi(net.get_bot(&id).is_some())
  });

  lua_api.add_dynamic_function("Net", "remove_bot", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.remove_bot(&id);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "get_bot_area", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow_mut();

    if let Some(bot) = net.get_bot(&id) {
      lua_ctx.pack_multi(bot.area_id.as_str())
    } else {
      Err(create_bot_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_bot_name", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow_mut();

    if let Some(bot) = net.get_bot(&id) {
      lua_ctx.pack_multi(bot.name.as_str())
    } else {
      Err(create_bot_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_bot_name", |api_ctx, lua_ctx, params| {
    let (id, name): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_bot_name(&id, name);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "get_bot_direction", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(bot) = net.get_bot(&id) {
      let direction_str = bot.direction.as_str();

      lua_ctx.pack_multi(direction_str)
    } else {
      Err(create_bot_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_bot_direction", |api_ctx, lua_ctx, params| {
    let (id, direction_string): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_bot_direction(&id, Direction::from(&direction_string));

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "get_bot_position", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(bot) = net.get_bot(&id) {
      let table = lua_ctx.create_table()?;
      table.set("x", bot.x)?;
      table.set("y", bot.y)?;
      table.set("z", bot.z)?;

      lua_ctx.pack_multi(table)
    } else {
      Err(create_bot_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "move_bot", |api_ctx, lua_ctx, params| {
    let (id, x, y, z): (String, f32, f32, f32) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.move_bot(&id, x, y, z);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "play_bot_animation", |api_ctx, lua_ctx, params| {
    let (id, name): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.play_bot_animation(&id, &name);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "set_bot_avatar", |api_ctx, lua_ctx, params| {
    let (id, texture_path, animation_path): (String, String, String) =
      lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_bot_avatar(&id, texture_path, animation_path);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "set_bot_emote", |api_ctx, lua_ctx, params| {
    let (id, emote_id): (String, u8) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_bot_emote(&id, emote_id);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "transfer_bot", |api_ctx, lua_ctx, params| {
    let (id, area_id, warp_in_option, x_option, y_option, z_option): (
      String,
      String,
      Option<bool>,
      Option<f32>,
      Option<f32>,
      Option<f32>,
    ) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();
    let warp_in = warp_in_option.unwrap_or(true);
    let x;
    let y;
    let z;

    if let Some(bot) = net.get_bot(&id) {
      x = x_option.unwrap_or(bot.x);
      y = y_option.unwrap_or(bot.y);
      z = z_option.unwrap_or(bot.z);
    } else {
      return Err(create_bot_error(&id));
    }

    net.transfer_bot(&id, &area_id, warp_in, x, y, z);

    lua_ctx.pack_multi(())
  });
}
