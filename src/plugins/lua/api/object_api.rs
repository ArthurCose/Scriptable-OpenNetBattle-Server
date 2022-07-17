use super::lua_errors::create_area_error;
use super::LuaApi;
use crate::net::map::{MapObject, MapObjectData, MapObjectSpecification, Tile};
use log::*;

pub fn inject_dynamic(lua_api: &mut LuaApi) {
  lua_api.add_dynamic_function("Net", "list_objects", |api_ctx, lua_ctx, params| {
    let area_id: mlua::String = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(area_id_str) {
      let result: Vec<u32> = area
        .get_map()
        .get_objects()
        .iter()
        .map(|object| object.id)
        .collect();

      lua_ctx.pack_multi(result)
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "get_object_by_id", |api_ctx, lua_ctx, params| {
    let (area_id, id): (mlua::String, u32) = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(area_id_str) {
      let optional_object = area.get_map().get_object_by_id(id);

      lua_ctx.pack_multi(map_optional_object_to_table(&lua_ctx, optional_object))
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "get_object_by_name", |api_ctx, lua_ctx, params| {
    let (area_id, name): (mlua::String, mlua::String) = lua_ctx.unpack_multi(params)?;
    let (area_id_str, name_str) = (area_id.to_str()?, name.to_str()?);

    let net = api_ctx.net_ref.borrow();

    if let Some(area) = net.get_area(area_id_str) {
      let optional_object = area.get_map().get_object_by_name(name_str);

      lua_ctx.pack_multi(map_optional_object_to_table(&lua_ctx, optional_object))
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "create_object", |api_ctx, lua_ctx, params| {
    let (area_id, table): (mlua::String, mlua::Table) = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    let area = if let Some(area) = net.get_area_mut(area_id_str) {
      area
    } else {
      return Err(create_area_error(area_id_str));
    };

    use std::collections::HashMap;

    let name: Option<String> = table.get("name")?;
    let class: Option<String> = table.get("class")?;
    let visible: Option<bool> = table.get("visible")?;
    let x: Option<f32> = table.get("x")?;
    let y: Option<f32> = table.get("y")?;
    let layer: Option<usize> = table.get("z")?;
    let width: Option<f32> = table.get("width")?;
    let height: Option<f32> = table.get("height")?;
    let rotation: Option<f32> = table.get("rotation")?;
    let data_table: mlua::Table = table.get("data")?;
    let custom_properties: Option<HashMap<String, String>> = table.get("custom_properties")?;

    let map = area.get_map_mut();

    let data = parse_object_data(data_table)?;

    let id = map.create_object(MapObjectSpecification {
      name: name.unwrap_or_default(),
      class: class.unwrap_or_default(),
      visible: visible.unwrap_or(true),
      x: x.unwrap_or_default(),
      y: y.unwrap_or_default(),
      layer: layer.unwrap_or_default(),
      width: width.unwrap_or_default(),
      height: height.unwrap_or_default(),
      rotation: rotation.unwrap_or_default(),
      data,
      custom_properties: custom_properties.unwrap_or_default(),
    });

    lua_ctx.pack_multi(id)
  });

  lua_api.add_dynamic_function("Net", "remove_object", |api_ctx, lua_ctx, params| {
    let (area_id, id): (mlua::String, u32) = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.remove_object(id);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_name", |api_ctx, lua_ctx, params| {
    let (area_id, id, name): (mlua::String, u32, String) = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.set_object_name(id, name);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_class", |api_ctx, lua_ctx, params| {
    let (area_id, id, class): (mlua::String, u32, String) = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.set_object_class(id, class);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_type", |api_ctx, lua_ctx, params| {
    warn!("Net.set_object_type() is deprecated, use Net.set_object_class()");

    let (area_id, id, class): (mlua::String, u32, String) = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.set_object_class(id, class);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function(
    "Net",
    "set_object_custom_property",
    |api_ctx, lua_ctx, params| {
      let (area_id, id, name, value): (mlua::String, u32, String, String) =
        lua_ctx.unpack_multi(params)?;
      let area_id_str = area_id.to_str()?;

      let mut net = api_ctx.net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(area_id_str) {
        let map = area.get_map_mut();

        map.set_object_custom_property(id, name, value);

        lua_ctx.pack_multi(())
      } else {
        Err(create_area_error(area_id_str))
      }
    },
  );

  lua_api.add_dynamic_function("Net", "resize_object", |api_ctx, lua_ctx, params| {
    let (area_id, id, width, height): (mlua::String, u32, f32, f32) =
      lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.resize_object(id, width, height);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_rotation", |api_ctx, lua_ctx, params| {
    let (area_id, id, rotation): (mlua::String, u32, f32) = lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.set_object_rotation(id, rotation);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function(
    "Net",
    "set_object_visibility",
    |api_ctx, lua_ctx, params| {
      let (area_id, id, visibility): (mlua::String, u32, bool) = lua_ctx.unpack_multi(params)?;
      let area_id_str = area_id.to_str()?;

      let mut net = api_ctx.net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(area_id_str) {
        let map = area.get_map_mut();

        map.set_object_visibility(id, visibility);

        lua_ctx.pack_multi(())
      } else {
        Err(create_area_error(area_id_str))
      }
    },
  );

  lua_api.add_dynamic_function("Net", "move_object", |api_ctx, lua_ctx, params| {
    let (area_id, id, x, y, layer): (mlua::String, u32, f32, f32, usize) =
      lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.move_object(id, x, y, layer);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });

  lua_api.add_dynamic_function("Net", "set_object_data", |api_ctx, lua_ctx, params| {
    let (area_id, id, data_table): (mlua::String, u32, mlua::Table) =
      lua_ctx.unpack_multi(params)?;
    let area_id_str = area_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(area) = net.get_area_mut(area_id_str) {
      let map = area.get_map_mut();

      map.set_object_data(id, parse_object_data(data_table)?);

      lua_ctx.pack_multi(())
    } else {
      Err(create_area_error(area_id_str))
    }
  });
}

fn parse_object_data(data_table: mlua::Table) -> mlua::Result<MapObjectData> {
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
    _ => return Err(mlua::Error::RuntimeError(String::from(
      "Invalid or missing type in data param. Accepted values: \"point\", \"rect\", \"ellipse\", \"polyline\", \"polygon\", \"tile\"",
    ))),
  };

  Ok(data)
}

fn extract_points_from_table(data_table: mlua::Table) -> mlua::Result<Vec<(f32, f32)>> {
  let points_table: Vec<mlua::Table> = data_table.get("points")?;

  let mut points = Vec::new();
  points.reserve(points_table.len());

  for point_table in points_table {
    let x = point_table.get("x")?;
    let y = point_table.get("y")?;

    points.push((x, y));
  }

  Ok(points)
}

fn map_optional_object_to_table<'lua>(
  lua_ctx: &'lua mlua::Lua,
  optional_object: Option<&MapObject>,
) -> Option<mlua::Table<'lua>> {
  let table = lua_ctx.create_table().ok()?;

  let object = optional_object?;

  table.set("id", object.id).ok()?;
  table.set("name", object.name.as_str()).ok()?;
  table.set("type", object.class.as_str()).ok()?; // todo: remove after deprecation ends
  table.set("class", object.class.as_str()).ok()?;
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
  lua_ctx: &'a mlua::Lua,
  points: &[(f32, f32)],
) -> mlua::Result<mlua::Table<'a>> {
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
