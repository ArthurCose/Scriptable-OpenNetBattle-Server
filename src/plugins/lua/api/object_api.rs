use super::lua_errors::create_area_error;
use crate::net::map::{MapObject, MapObjectData, Tile};
use crate::net::Net;
use std::cell::RefCell;

#[allow(clippy::type_complexity)]
pub fn add_object_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "list_objects",
    scope.create_function(move |_, area_id: String| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let result: Vec<u32> = area
          .get_map()
          .get_objects()
          .iter()
          .map(|object| object.id)
          .collect();

        Ok(result)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_object_by_id",
    scope.create_function(move |lua_ctx, (area_id, id): (String, u32)| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let optional_object = area.get_map().get_object_by_id(id);

        Ok(map_optional_object_to_table(&lua_ctx, optional_object))
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_object_by_name",
    scope.create_function(move |lua_ctx, (area_id, name): (String, String)| {
      let net = net_ref.borrow();

      if let Some(area) = net.get_area(&area_id) {
        let optional_object = area.get_map().get_object_by_name(&name);

        Ok(map_optional_object_to_table(&lua_ctx, optional_object))
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "create_object",
    scope.create_function(
      move |_,
            (area_id, name, object_type, x, y, layer, width, height, rotation, data_table): (
        String,
        String,
        String,
        f32,
        f32,
        usize,
        f32,
        f32,
        f32,
        rlua::Table,
      )| {
        let mut net = net_ref.borrow_mut();

        if let Some(area) = net.get_area_mut(&area_id) {
          let map = area.get_map_mut();

          let data = if data_table.contains_key("gid")? {
            let flipped_horizontally: Option<bool> = data_table.get("flippedHorizontally")?;
            let flipped_vertically: Option<bool> = data_table.get("flippedVertically")?;

            let tile = Tile {
              gid: data_table.get("gid")?,
              flipped_horizontally: flipped_horizontally.unwrap_or_default(),
              flipped_vertically: flipped_vertically.unwrap_or_default(),
              flipped_anti_diagonally: false,
            };

            MapObjectData::TileObject { tile }
          } else if data_table.contains_key("points")? {
            let point_tables: Vec<rlua::Table> = data_table.get("points")?;
            let mut points = Vec::new();
            points.reserve(point_tables.len());

            for point_table in point_tables {
              let x = point_table.get("x")?;
              let y = point_table.get("y")?;

              points.push((x, y));
            }

            MapObjectData::Polygon { points }
          } else if width != 0.0 || height != 0.0 {
            MapObjectData::Rect
          } else {
            MapObjectData::Point
          };

          let id = map.create_object(
            name,
            object_type,
            x,
            y,
            layer,
            width,
            height,
            rotation,
            data,
          );

          Ok(id)
        } else {
          Err(create_area_error(&area_id))
        }
      },
    )?,
  )?;

  api_table.set(
    "remove_object",
    scope.create_function(move |_, (area_id, id): (String, u32)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.remove_object(id);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_object_name",
    scope.create_function(move |_, (area_id, id, name): (String, u32, String)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_object_name(id, name);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "resize_object",
    scope.create_function(
      move |_, (area_id, id, width, height): (String, u32, f32, f32)| {
        let mut net = net_ref.borrow_mut();

        if let Some(area) = net.get_area_mut(&area_id) {
          let map = area.get_map_mut();

          map.resize_object(id, width, height);

          Ok(())
        } else {
          Err(create_area_error(&area_id))
        }
      },
    )?,
  )?;

  api_table.set(
    "set_object_rotation",
    scope.create_function(move |_, (area_id, id, rotation): (String, u32, f32)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_object_rotation(id, rotation);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_object_visibility",
    scope.create_function(move |_, (area_id, id, visibility): (String, u32, bool)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_object_visibility(id, visibility);

        Ok(())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "move_object",
    scope.create_function(
      move |_, (area_id, id, x, y, layer): (String, u32, f32, f32, usize)| {
        let mut net = net_ref.borrow_mut();

        if let Some(area) = net.get_area_mut(&area_id) {
          let map = area.get_map_mut();

          map.move_object(id, x, y, layer);

          Ok(())
        } else {
          Err(create_area_error(&area_id))
        }
      },
    )?,
  )?;

  Ok(())
}

fn map_optional_object_to_table<'a>(
  lua_ctx: &rlua::Context<'a>,
  optional_object: Option<&MapObject>,
) -> Option<rlua::Table<'a>> {
  let table = lua_ctx.create_table().ok()?;

  let object = optional_object?;

  table.set("id", object.id).ok()?;
  table.set("name", object.name.clone()).ok()?;
  table.set("type", object.object_type.clone()).ok()?;
  table.set("visible", object.visible).ok()?;
  table.set("x", object.x).ok()?;
  table.set("y", object.y).ok()?;
  table.set("z", object.layer).ok()?;
  table.set("width", object.width).ok()?;
  table.set("height", object.height).ok()?;
  table.set("rotation", object.rotation).ok()?;

  let data_table = lua_ctx.create_table().ok()?;

  match &object.data {
    MapObjectData::Polygon { points } => {
      let points_table = lua_ctx.create_table().ok()?;

      // lua lists start at 1
      let mut i = 1;

      for point in points {
        let point_table = lua_ctx.create_table().ok()?;
        point_table.set("x", point.0).ok()?;
        point_table.set("y", point.1).ok()?;

        points_table.set(i, point_table).ok()?;
        i += 1;
      }

      data_table.set("points", points_table).ok()?;
      Some(())
    }
    MapObjectData::TileObject { tile } => {
      data_table.set("gid", tile.gid).ok()?;
      data_table
        .set("flippedHorizontally", tile.flipped_horizontally)
        .ok()?;
      data_table
        .set("flippedVertically", tile.flipped_vertically)
        .ok()?;
      data_table.set("rotated", false).ok()?;
      Some(())
    }
    _ => Some(()),
  }?;

  table.set("data", data_table).ok()?;

  // todo: properties

  Some(table)
}
