use super::lua_errors::create_area_error;
use super::LuaAPI;
use crate::net::map::{MapObject, MapObjectData, Tile};

pub fn inject_dynamic(lua_api: &mut LuaAPI) {
  lua_api.add_dynamic_function("Net", "list_objects", |api_ctx, lua_ctx, params| {
    let area_id: String = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let result: Vec<u32> = area
        .get_map()
        .get_objects()
        .iter()
        .map(|object| object.id)
        .collect();

      lua_ctx.pack_multi(result)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_object_by_id", |api_ctx, lua_ctx, params| {
    let (area_id, id): (String, u32) = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let optional_object = area.get_map().get_object_by_id(id);

      lua_ctx.pack_multi(map_optional_object_to_table(&lua_ctx, optional_object))
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "get_object_by_name", |api_ctx, lua_ctx, params| {
    let (area_id, name): (String, String) = lua_ctx.unpack_multi(params)?;
    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(&area_id) {
      let optional_object = area.get_map().get_object_by_name(&name);

      lua_ctx.pack_multi(map_optional_object_to_table(&lua_ctx, optional_object))
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "create_object", |api_ctx, lua_ctx, params| {
    let (area_id, name, object_type, x, y, layer, width, height, rotation, data_table): (
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
    ) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      let data = parse_object_data(data_table)?;

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

      lua_ctx.pack_multi(id)
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "remove_object", |api_ctx, lua_ctx, params| {
    let (area_id, id): (String, u32) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.remove_object(id);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_name", |api_ctx, lua_ctx, params| {
    let (area_id, id, name): (String, u32, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_object_name(id, name);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_type", |api_ctx, lua_ctx, params| {
    let (area_id, id, object_type): (String, u32, String) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_object_type(id, object_type);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function(
    "Net",
    "set_object_custom_property",
    |api_ctx, lua_ctx, params| {
      let (area_id, id, name, value): (String, u32, String, String) =
        lua_ctx.unpack_multi(params)?;
      let mut net = api_ctx.net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_object_custom_property(id, name, value);

        lua_ctx.pack_multi(())
      } else {
        Err(create_area_error(&area_id))
      }
    },
  );

  lua_api.add_dynamic_function("Net", "resize_object", |api_ctx, lua_ctx, params| {
    let (area_id, id, width, height): (String, u32, f32, f32) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.resize_object(id, width, height);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_rotation", |api_ctx, lua_ctx, params| {
    let (area_id, id, rotation): (String, u32, f32) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_object_rotation(id, rotation);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function(
    "Net",
    "set_object_visibility",
    |api_ctx, lua_ctx, params| {
      let (area_id, id, visibility): (String, u32, bool) = lua_ctx.unpack_multi(params)?;
      let mut net = api_ctx.net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let map = area.get_map_mut();

        map.set_object_visibility(id, visibility);

        lua_ctx.pack_multi(())
      } else {
        Err(create_area_error(&area_id))
      }
    },
  );

  lua_api.add_dynamic_function("Net", "move_object", |api_ctx, lua_ctx, params| {
    let (area_id, id, x, y, layer): (String, u32, f32, f32, usize) =
      lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.move_object(id, x, y, layer);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_data", |api_ctx, lua_ctx, params| {
    let (area_id, id, data_table): (String, u32, rlua::Table) = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(&area_id) {
      let map = area.get_map_mut();

      map.set_object_data(id, parse_object_data(data_table)?);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(&area_id))
    }
  });
}

fn parse_object_data(data_table: rlua::Table) -> rlua::Result<MapObjectData> {
  let object_type: String = data_table.get("type")?;

  let data = match object_type.as_str() {
    "point" => MapObjectData::Point,
    "rect" => MapObjectData::Rect,
    "ellipse" => MapObjectData::Ellipse,
    "polyline" => {
      let points = extract_points_from_table(data_table)?;

      MapObjectData::Polyline { points }
    }
    "polygon" => {
      let points = extract_points_from_table(data_table)?;

      MapObjectData::Polygon { points }
    }
    "tile" => {
      let flipped_horizontally: Option<bool> = data_table.get("flipped_horizontally")?;
      let flipped_vertically: Option<bool> = data_table.get("flipped_vertically")?;

      let tile = Tile {
        gid: data_table.get("gid")?,
        flipped_horizontally: flipped_horizontally.unwrap_or_default(),
        flipped_vertically: flipped_vertically.unwrap_or_default(),
        flipped_anti_diagonally: false,
      };

      MapObjectData::TileObject { tile }
    }
    _ => Err(rlua::Error::RuntimeError(String::from(
      "Invalid or missing type in data param. Accepted values: \"point\", \"rect\", \"ellipse\", \"polyline\", \"polygon\", \"tile\"",
    )))?,
  };

  Ok(data)
}

fn extract_points_from_table(data_table: rlua::Table) -> rlua::Result<Vec<(f32, f32)>> {
  let points_table: Vec<rlua::Table> = data_table.get("points")?;

  let mut points = Vec::new();
  points.reserve(points_table.len());

  for point_table in points_table {
    let x = point_table.get("x")?;
    let y = point_table.get("y")?;

    points.push((x, y));
  }

  Ok(points)
}

fn map_optional_object_to_table<'a>(
  lua_ctx: &rlua::Context<'a>,
  optional_object: Option<&MapObject>,
) -> Option<rlua::Table<'a>> {
  let table = lua_ctx.create_table().ok()?;

  let object = optional_object?;

  table.set("id", object.id).ok()?;
  table.set("name", object.name.as_str()).ok()?;
  table.set("type", object.object_type.as_str()).ok()?;
  table.set("visible", object.visible).ok()?;
  table.set("x", object.x).ok()?;
  table.set("y", object.y).ok()?;
  table.set("z", object.layer).ok()?;
  table.set("width", object.width).ok()?;
  table.set("height", object.height).ok()?;
  table.set("rotation", object.rotation).ok()?;

  let data_table = lua_ctx.create_table().ok()?;

  match &object.data {
    MapObjectData::Point => {
      data_table.set("type", "point").ok()?;
    }
    MapObjectData::Rect => {
      data_table.set("type", "rect").ok()?;
    }
    MapObjectData::Ellipse => {
      data_table.set("type", "ellipse").ok()?;
    }
    MapObjectData::Polyline { points } => {
      data_table.set("type", "polyline").ok()?;

      let points_table = points_to_table(lua_ctx, points).ok()?;

      data_table.set("points", points_table).ok()?;
    }
    MapObjectData::Polygon { points } => {
      data_table.set("type", "polygon").ok()?;

      let points_table = points_to_table(lua_ctx, points).ok()?;

      data_table.set("points", points_table).ok()?;
    }
    MapObjectData::TileObject { tile } => {
      data_table.set("type", "tile").ok()?;
      data_table.set("gid", tile.gid).ok()?;
      data_table
        .set("flipped_horizontally", tile.flipped_horizontally)
        .ok()?;
      data_table
        .set("flipped_vertically", tile.flipped_vertically)
        .ok()?;
      data_table.set("rotated", false).ok()?;
    }
  };

  table.set("data", data_table).ok()?;

  let custom_properties_table = lua_ctx.create_table().ok()?;

  for (name, value) in &object.custom_properties {
    custom_properties_table
      .set(name.as_str(), value.as_str())
      .ok()?
  }

  table
    .set("custom_properties", custom_properties_table)
    .ok()?;

  Some(table)
}

fn points_to_table<'a>(
  lua_ctx: &rlua::Context<'a>,
  points: &[(f32, f32)],
) -> rlua::Result<rlua::Table<'a>> {
  let points_table = lua_ctx.create_table()?;

  // lua lists start at 1
  let mut i = 1;

  for point in points {
    let point_table = lua_ctx.create_table()?;
    point_table.set("x", point.0)?;
    point_table.set("y", point.1)?;

    points_table.set(i, point_table)?;
    i += 1;
  }

  Ok(points_table)
}
