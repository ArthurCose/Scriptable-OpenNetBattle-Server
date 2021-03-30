use super::lua_errors::{create_area_error, create_player_error};
use super::LuaAPI;
use crate::net::Direction;

#[allow(clippy::type_complexity)]
pub fn inject_dynamic(lua_api: &mut LuaAPI) {
  lua_api.add_dynamic_function("Net", "list_players", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let connected_players = area.get_connected_players();
      let result: Vec<String> = connected_players.to_vec();

      lua_ctx.pack_multi(result)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "is_player", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    lua_ctx.pack_multi(net.get_player(&id).is_some())
  });

  lua_api.add_dynamic_function("Net", "get_player_area", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow_mut();

    if let Some(player) = net.get_player(&id) {
      lua_ctx.pack_multi(player.area_id.as_str())
    } else {
      Err(create_player_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_player_name", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow_mut();

    if let Some(player) = net.get_player(&id) {
      lua_ctx.pack_multi(player.name.as_str())
    } else {
      Err(create_player_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_player_name", |api_ctx, lua_ctx, params| {
    let (id, name): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_player_name(&id, name);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "get_player_position", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(player) = net.get_player(&id) {
      let table = lua_ctx.create_table()?;
      table.set("x", player.x)?;
      table.set("y", player.y)?;
      table.set("z", player.z)?;

      lua_ctx.pack_multi(table)
    } else {
      Err(create_player_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_player_direction", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(player) = net.get_player(&id) {
      let direction_str = player.direction.as_str();

      lua_ctx.pack_multi(direction_str)
    } else {
      Err(create_player_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_player_mugshot", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(player) = net.get_player(&id) {
      let table = lua_ctx.create_table()?;
      table.set("texture_path", player.mugshot_texture_path.as_str())?;
      table.set("animation_path", player.mugshot_animation_path.as_str())?;

      lua_ctx.pack_multi(table)
    } else {
      Err(create_player_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_player_avatar", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(player) = net.get_player(&id) {
      let table = lua_ctx.create_table()?;
      table.set("texture_path", player.texture_path.as_str())?;
      table.set("animation_path", player.animation_path.as_str())?;

      lua_ctx.pack_multi(table)
    } else {
      Err(create_player_error(&id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_player_avatar", |api_ctx, lua_ctx, params| {
    let (id, texture_path, animation_path): (String, String, String) =
      lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.set_player_avatar(&id, texture_path, animation_path);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "is_player_in_widget", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    let is_in_widget = net.is_player_in_widget(&id);

    lua_ctx.pack_multi(is_in_widget)
  });

  lua_api.add_dynamic_function(
    "Net",
    "exclude_object_for_player",
    |api_ctx, lua_ctx, params| {
      let (id, object_id): (String, u32) = lua_ctx.unpack_multi(params)?;
      let mut net = api_ctx.net_ref.borrow_mut();

      net.exclude_object_for_player(&id, object_id);

      lua_ctx.pack_multi(())
    },
  );

  lua_api.add_dynamic_function(
    "Net",
    "include_object_for_player",
    |api_ctx, lua_ctx, params| {
      let (id, object_id): (String, u32) = lua_ctx.unpack_multi(params)?;
      let mut net = api_ctx.net_ref.borrow_mut();

      net.include_object_for_player(&id, object_id);

      lua_ctx.pack_multi(())
    },
  );

  lua_api.add_dynamic_function("Net", "move_player_camera", |api_ctx, lua_ctx, params| {
    let (id, x, y, z, duration): (String, f32, f32, f32, Option<f32>) =
      lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.move_player_camera(&id, x, y, z, duration.unwrap_or_default());

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "slide_player_camera", |api_ctx, lua_ctx, params| {
    let (id, x, y, z, duration): (String, f32, f32, f32, f32) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.slide_player_camera(&id, x, y, z, duration);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "unlock_player_camera", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.unlock_player_camera(&id);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "lock_player_input", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.lock_player_input(&id);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "unlock_player_input", |api_ctx, lua_ctx, params| {
    let id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.unlock_player_input(&id);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "move_player", |api_ctx, lua_ctx, params| {
    let (id, x, y, z): (String, f32, f32, f32) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.move_player(&id, x, y, z);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "message_player", |api_ctx, lua_ctx, params| {
    let (id, message, mug_texture_path, mug_animation_path): (
      String,
      String,
      Option<String>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();

    api_ctx
      .message_tracker_ref
      .borrow_mut()
      .track_message(&id, api_ctx.script_dir.clone());

    net.message_player(
      &id,
      &message,
      &mug_texture_path.unwrap_or_default(),
      &mug_animation_path.unwrap_or_default(),
    );

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "question_player", |api_ctx, lua_ctx, params| {
    let (id, message, mug_texture_path, mug_animation_path): (
      String,
      String,
      Option<String>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();

    api_ctx
      .message_tracker_ref
      .borrow_mut()
      .track_message(&id, api_ctx.script_dir.clone());

    net.question_player(
      &id,
      &message,
      &mug_texture_path.unwrap_or_default(),
      &mug_animation_path.unwrap_or_default(),
    );

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "quiz_player", |api_ctx, lua_ctx, params| {
    let (id, option_a, option_b, option_c, mug_texture_path, mug_animation_path): (
      String,
      Option<String>,
      Option<String>,
      Option<String>,
      Option<String>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();

    api_ctx
      .message_tracker_ref
      .borrow_mut()
      .track_message(&id, api_ctx.script_dir.clone());

    net.quiz_player(
      &id,
      &option_a.unwrap_or_default(),
      &option_b.unwrap_or_default(),
      &option_c.unwrap_or_default(),
      &mug_texture_path.unwrap_or_default(),
      &mug_animation_path.unwrap_or_default(),
    );

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "transfer_player", |api_ctx, lua_ctx, params| {
    let (id, area_id, warp_in_option, x_option, y_option, z_option, direction_option): (
      String,
      String,
      Option<bool>,
      Option<f32>,
      Option<f32>,
      Option<f32>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();
    let warp_in = warp_in_option.unwrap_or(true);
    let x;
    let y;
    let z;

    if let Some(player) = net.get_player(&id) {
      x = x_option.unwrap_or(player.x);
      y = y_option.unwrap_or(player.y);
      z = z_option.unwrap_or(player.z);
    } else {
      return Err(create_player_error(&id));
    }

    let direction = Direction::from(direction_option.unwrap_or_default().as_str());

    net.transfer_player(&id, &area_id, warp_in, x, y, z, direction);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "kick_player", |api_ctx, lua_ctx, params| {
    let (id, reason): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.kick_player(&id, &reason);

    lua_ctx.pack_multi(())
  });
}
