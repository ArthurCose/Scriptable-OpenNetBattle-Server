use super::lua_errors::create_area_error;
use crate::net::map::Tile;
use crate::net::Net;
use std::cell::RefCell;

#[allow(clippy::type_complexity)]
pub fn add_area_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "list_areas",
    scope.create_function(move |_, _: ()| {
      let net = net_ref.borrow();

      let area_ids: Vec<String> = net
        .get_areas()
        .map(|area| area.get_id().to_string())
        .collect();

      Ok(area_ids)
    })?,
  )?;

  api_table.set(
    "clone_area",
    scope.create_function(move |_, (area_id, new_id): (String, String)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area(&area_id) {
        let map = area.get_map().clone();

        net.add_area(new_id, map);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "remove_area",
    scope.create_function(move |_, area_id: String| {
      let mut net = net_ref.borrow_mut();

      net.remove_area(&area_id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "get_width",
    scope.create_function(move |_, area_id: String| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        Ok(area.get_map_mut().get_width())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_height",
    scope.create_function(move |_, area_id: String| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        Ok(area.get_map_mut().get_height())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_area_name",
    scope.create_function(move |_, area_id: String| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        Ok(area.get_map().get_name().clone())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_area_name",
    scope.create_function(move |_, (area_id, name): (String, String)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_name(name);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_song",
    scope.create_function(move |_, area_id: String| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        Ok(area.get_map().get_song_path().clone())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_song",
    scope.create_function(move |_, (area_id, path): (String, String)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_song_path(path);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_background_name",
    scope.create_function(move |_, area_id: String| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        Ok(area.get_map().get_background_name().clone())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_background",
    scope.create_function(move |_, (area_id, name): (String, String)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_background_name(name);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_custom_background",
    scope.create_function(move |lua_ctx, area_id: String| {
      let net = net_ref.borrow();

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

        Ok(table)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_custom_background_velocity",
    scope.create_function(move |lua_ctx, area_id: String| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let map = area.get_map();

        let (vel_x, vel_y) = map.get_custom_background_velocity();

        let table = lua_ctx.create_table()?;
        table.set("x", vel_x)?;
        table.set("y", vel_y)?;

        Ok(table)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_custom_background",
    scope.create_function(
      move |_,
            (area_id, texture_path, animation_path, vel_x, vel_y): (
        String,
        String,
        Option<String>,
        Option<f32>,
        Option<f32>,
      )| {
        let mut net = net_ref.borrow_mut();

        if let Some(area) = net.get_area_mut(&area_id) {
          let map = area.get_map_mut();

          map.set_background_name(String::from("custom"));
          map.set_custom_background_texture_path(texture_path);
          map.set_custom_background_animation_path(animation_path.unwrap_or_default());
          map.set_custom_background_velocity(vel_x.unwrap_or_default(), vel_y.unwrap_or_default());

          Ok(())
        } else {
          Err(create_area_error(&area_id))
        }
      },
    )?,
  )?;

  api_table.set(
    "get_spawn_position",
    scope.create_function(move |lua_ctx, area_id: String| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let (x, y, z) = area.get_map().get_spawn();

        let table = lua_ctx.create_table()?;
        table.set("x", x)?;
        table.set("y", y)?;
        table.set("z", z)?;

        Ok(table)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_spawn_position",
    scope.create_function(move |_, (area_id, x, y, z): (String, f32, f32, f32)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_spawn(x, y, z);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "list_tilesets",
    scope.create_function(move |_, area_id: String| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let map = area.get_map();
        let tilesets = map.get_tilesets();
        let tileset_paths: Vec<String> = tilesets
          .iter()
          .map(|tileset| tileset.path.clone())
          .collect();

        Ok(tileset_paths)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_tileset",
    scope.create_function(move |lua_ctx, (area_id, path): (String, String)| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let map = area.get_map();
        let tilesets = map.get_tilesets();
        let optional_tileset = tilesets.iter().find(|tileset| tileset.path == path);

        if let Some(tileset) = optional_tileset {
          let table = lua_ctx.create_table()?;
          table.set("path", tileset.path.clone())?;
          table.set("firstGid", tileset.first_gid)?;

          return Ok(Some(table));
        }

        Ok(None)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_tileset_for_tile",
    scope.create_function(move |lua_ctx, (area_id, tile_gid): (String, u32)| {
      let net = net_ref.borrow();

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

          return Ok(Some(table));
        }

        Ok(None)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_tile",
    scope.create_function(
      move |lua_ctx, (area_id, x, y, z): (String, usize, usize, usize)| {
        let net = net_ref.borrow();

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

          Ok(table)
        } else {
          Err(create_area_error(&area_id))
        }
      },
    )?,
  )?;

  api_table.set(
    "set_tile",
    scope.create_function(
      move |_,
            (area_id, x, y, z, gid, flip_horizontal, flip_vertical, rotate): (
        String,
        usize,
        usize,
        usize,
        u32,
        Option<bool>,
        Option<bool>,
        Option<bool>,
      )| {
        let mut net = net_ref.borrow_mut();

        if let Some(area) = net.get_area_mut(&area_id) {
          let tile = Tile {
            gid,
            flipped_horizontally: flip_horizontal.unwrap_or(false),
            flipped_vertically: flip_vertical.unwrap_or(false),
            flipped_anti_diagonally: rotate.unwrap_or(false),
          };

          area.get_map_mut().set_tile(x, y, z, tile);
          Ok(())
        } else {
          Err(create_area_error(&area_id))
        }
      },
    )?,
  )?;

  Ok(())
}
