use super::lua_errors::{create_area_error, create_bot_error};
use super::LuaAPI;
use crate::net::{Direction, Navi};

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
    let (id, name, area_id, texture_path, animation_path, x, y, z, solid): (
      String,
      String,
      String,
      String,
      String,
      f32,
      f32,
      f32,
      Option<bool>,
    ) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if net.get_area_mut(&area_id).is_some() {
      let bot = Navi {
        id,
        name,
        area_id,
        texture_path,
        animation_path,
        direction: Direction::None,
        x,
        y,
        z,
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
      lua_ctx.pack_multi(bot.area_id.clone())
    } else {
      Err(create_bot_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_bot_name", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow_mut();

    if let Some(bot) = net.get_bot(&id) {
      lua_ctx.pack_multi(bot.name.clone())
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
