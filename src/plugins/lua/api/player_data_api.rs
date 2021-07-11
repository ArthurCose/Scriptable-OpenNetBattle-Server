use super::lua_errors::create_player_error;
use super::LuaApi;

pub fn inject_dynamic(lua_api: &mut LuaApi) {
  lua_api.add_dynamic_function("Net", "get_player_secret", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(player_data) = &net.get_player_data(player_id_str) {
      lua_ctx.pack_multi(player_data.identity.as_str())
    } else {
      Err(create_player_error(player_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "get_player_health", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(player_data) = &net.get_player_data(player_id_str) {
      lua_ctx.pack_multi(player_data.health)
    } else {
      Err(create_player_error(player_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_player_health", |api_ctx, lua_ctx, params| {
    let (player_id, health): (rlua::String, u32) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_player_health(player_id_str, health);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function(
    "Net",
    "get_player_max_health",
    |api_ctx, lua_ctx, params| {
      let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
      let player_id_str = player_id.to_str()?;

      let net = api_ctx.net_ref.borrow();

      if let Some(player_data) = &net.get_player_data(player_id_str) {
        lua_ctx.pack_multi(player_data.max_health)
      } else {
        Err(create_player_error(player_id_str))
      }
    },
  );

  lua_api.add_dynamic_function(
    "Net",
    "set_player_max_health",
    |api_ctx, lua_ctx, params| {
      let (player_id, max_health): (rlua::String, u32) = lua_ctx.unpack_multi(params)?;
      let player_id_str = player_id.to_str()?;

      let mut net = api_ctx.net_ref.borrow_mut();

      net.set_player_max_health(player_id_str, max_health);

      lua_ctx.pack_multi(())
    },
  );

  lua_api.add_dynamic_function("Net", "get_player_emotion", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(player_data) = &net.get_player_data(player_id_str) {
      lua_ctx.pack_multi(player_data.emotion)
    } else {
      Err(create_player_error(player_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_player_emotion", |api_ctx, lua_ctx, params| {
    let (player_id, emotion): (rlua::String, u8) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_player_emotion(player_id_str, emotion);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "get_player_money", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(player_data) = &net.get_player_data(player_id_str) {
      lua_ctx.pack_multi(player_data.money)
    } else {
      Err(create_player_error(player_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_player_money", |api_ctx, lua_ctx, params| {
    let (player_id, money): (rlua::String, u32) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_player_money(player_id_str, money);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "get_player_items", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(player_data) = &net.get_player_data(player_id_str) {
      lua_ctx.pack_multi(player_data.items.clone())
    } else {
      Err(create_player_error(player_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "give_player_item", |api_ctx, lua_ctx, params| {
    let (player_id, name): (rlua::String, String) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    net.give_player_item(player_id_str, name);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "remove_player_item", |api_ctx, lua_ctx, params| {
    let (player_id, name): (rlua::String, rlua::String) = lua_ctx.unpack_multi(params)?;
    let (player_id_str, name_str) = (player_id.to_str()?, name.to_str()?);

    let mut net = api_ctx.net_ref.borrow_mut();

    net.remove_player_item(player_id_str, name_str);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "player_has_item", |api_ctx, lua_ctx, params| {
    let (player_id, name): (rlua::String, String) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(player_data) = &net.get_player_data(player_id_str) {
      lua_ctx.pack_multi(player_data.items.contains(&name))
    } else {
      Err(create_player_error(player_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "get_item_description", |api_ctx, lua_ctx, params| {
    let name: rlua::String = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();

    net.get_item_description(name.to_str()?);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "set_item_description", |api_ctx, lua_ctx, params| {
    let (name, description): (String, String) = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_item_description(name, description);

    lua_ctx.pack_multi(())
  });
}
