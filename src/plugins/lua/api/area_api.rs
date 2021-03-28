use super::lua_errors::create_area_error;
use super::LuaAPI;
use crate::net::map::{Map, Tile};
use crate::net::Direction;

#[allow(clippy::type_complexity)]
pub fn inject_dynamic(lua_api: &mut LuaAPI) {
  lua_api.add_dynamic_function("Net", "list_areas", |api_ctx, lua_ctx, _| {
    let net = api_ctx.net_ref.borrow();

    let area_ids: Vec<String> = net
      .get_areas()
      .map(|area| area.get_id().to_string())
      .collect();

    lua_ctx.pack_multi(area_ids)
  });

  lua_api.add_dynamic_function("Net", "update_area", |api_ctx, lua_ctx, params| {
    let (area_id, data): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    let map = Map::from(data);

    if let Some(area) = net.get_area_mut(&area_id) {
      area.set_map(map);
    } else {
      net.add_area(area_id, map);
    }

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "clone_area", |api_ctx, lua_ctx, params| {
    let (area_id, new_id): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area(&area_id) {
      let map = area.get_map().clone();

      net.add_area(new_id, map);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "map_to_string", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();
      lua_ctx.pack_multi(map.render())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "remove_area", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    net.remove_area(&area_id);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "get_width", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      lua_ctx.pack_multi(area.get_map_mut().get_width())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_height", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      lua_ctx.pack_multi(area.get_map_mut().get_height())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_tile_width", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      lua_ctx.pack_multi(area.get_map_mut().get_tile_width())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_tile_height", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      lua_ctx.pack_multi(area.get_map_mut().get_tile_height())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_area_name", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      lua_ctx.pack_multi(area.get_map().get_name().clone())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_area_name", |api_ctx, lua_ctx, params| {
    let (area_id, name): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_name(name);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_song", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      lua_ctx.pack_multi(area.get_map().get_song_path().clone())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_song", |api_ctx, lua_ctx, params| {
    let (area_id, path): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_song_path(path);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_background_name", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      lua_ctx.pack_multi(area.get_map().get_background_name().clone())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_background", |api_ctx, lua_ctx, params| {
    let (area_id, name): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_background_name(name);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function(
    "Net",
    "get_custom_background",
    |api_ctx, lua_ctx, params| {
      let area_id: String = lua_ctx.unpack_multi(params)?;
      let net = api_ctx.net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let map = area.get_map();

        let table = lua_ctx.create_table()?;

        table.set(
          "texturePath",
          map.get_custom_background_texture_path().clone(),
        )?;

        table.set(
          "animationPath",
          map.get_custom_background_animation_path().clone(),
        )?;

        lua_ctx.pack_multi(table)
      } else {
        Err(create_area_error(&area_id))
      }
    },
  );

  lua_api.add_dynamic_function(
    "Net",
    "get_custom_background_velocity",
    |api_ctx, lua_ctx, params| {
      let area_id: String = lua_ctx.unpack_multi(params)?;
      let net = api_ctx.net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let map = area.get_map();

        let (vel_x, vel_y) = map.get_custom_background_velocity();

        let table = lua_ctx.create_table()?;
        table.set("x", vel_x)?;
        table.set("y", vel_y)?;

        lua_ctx.pack_multi(table)
      } else {
        Err(create_area_error(&area_id))
      }
    },
  );

  lua_api.add_dynamic_function(
    "Net",
    "set_custom_background",
    |api_ctx, lua_ctx, params| {
      let (area_id, texture_path, animation_path, vel_x, vel_y): (
        String,
        String,
        Option<String>,
        Option<f32>,
        Option<f32>,
      ) = lua_ctx.unpack_multi(params)?;

      let mut net = api_ctx.net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_background_name(String::from("custom"));
        map.set_custom_background_texture_path(texture_path);
        map.set_custom_background_animation_path(animation_path.unwrap_or_default());
        map.set_custom_background_velocity(vel_x.unwrap_or_default(), vel_y.unwrap_or_default());

        lua_ctx.pack_multi(())
      } else {
        Err(create_area_error(&area_id))
      }
    },
  );

  lua_api.add_dynamic_function("Net", "get_spawn_position", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let (x, y, z) = area.get_map().get_spawn();

      let table = lua_ctx.create_table()?;
      table.set("x", x)?;
      table.set("y", y)?;
      table.set("z", z)?;

      lua_ctx.pack_multi(table)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_spawn_position", |api_ctx, lua_ctx, params| {
    let (area_id, x, y, z): (String, f32, f32, f32) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_spawn(x, y, z);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_spawn_direction", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let direction = area.get_map().get_spawn_direction();

      lua_ctx.pack_multi(direction.to_string())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_spawn_direction", |api_ctx, lua_ctx, params| {
    let (area_id, direction_string): (String, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      let direction = Direction::from(direction_string.as_str());
      map.set_spawn_direction(direction);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "list_tilesets", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let map = area.get_map();
      let tilesets = map.get_tilesets();
      let tileset_paths: Vec<String> = tilesets
        .iter()
        .map(|tileset| tileset.path.clone())
        .collect();

      lua_ctx.pack_multi(tileset_paths)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_tileset", |api_ctx, lua_ctx, params| {
    let (area_id, path): (String, String) = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let map = area.get_map();
      let tilesets = map.get_tilesets();
      let optional_tileset = tilesets.iter().find(|tileset| tileset.path == path);

      if let Some(tileset) = optional_tileset {
        let table = lua_ctx.create_table()?;
        table.set("path", tileset.path.clone())?;
        table.set("firstGid", tileset.first_gid)?;

        return lua_ctx.pack_multi(Some(table));
      }

      lua_ctx.pack_multi(rlua::Nil)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_tileset_for_tile", |api_ctx, lua_ctx, params| {
    let (area_id, tile_gid): (String, u32) = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let map = area.get_map();
      let tilesets = map.get_tilesets();
      let optional_tileset = tilesets
        .iter()
        .take_while(|tileset| tileset.first_gid <= tile_gid)
        .last();

      if let Some(tileset) = optional_tileset {
        let table = lua_ctx.create_table()?;
        table.set("path", tileset.path.clone())?;
        table.set("firstGid", tileset.first_gid)?;

        return lua_ctx.pack_multi(Some(table));
      }

      lua_ctx.pack_multi(rlua::Nil)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_tile", |api_ctx, lua_ctx, params| {
    let (area_id, x, y, z): (String, usize, usize, usize) = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let tile = area.get_map().get_tile(x, y, z);

      let table = lua_ctx.create_table()?;

      table.set("gid", tile.gid)?;

      if tile.flipped_anti_diagonally {
        table.set("flippedHorizontal", tile.flipped_vertically)?;
        table.set("flippedVertical", !tile.flipped_horizontally)?;
      } else {
        table.set("flippedHorizontal", tile.flipped_horizontally)?;
        table.set("flippedVertical", tile.flipped_vertically)?;
      }

      table.set("rotated", tile.flipped_anti_diagonally)?;

      lua_ctx.pack_multi(table)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_tile", |api_ctx, lua_ctx, params| {
    let (area_id, x, y, z, gid, flip_horizontal, flip_vertical, rotate): (
      String,
      usize,
      usize,
      usize,
      u32,
      Option<bool>,
      Option<bool>,
      Option<bool>,
    ) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let tile = Tile {
        gid,
        flipped_horizontally: flip_horizontal.unwrap_or(false),
        flipped_vertically: flip_vertical.unwrap_or(false),
        flipped_anti_diagonally: rotate.unwrap_or(false),
      };

      area.get_map_mut().set_tile(x, y, z, tile);
      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });
}
