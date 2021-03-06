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
    "get_tile_gid",
    scope.create_function(
      move |_, (area_id, x, y, z): (String, usize, usize, usize)| {
        let mut net = net_ref.borrow_mut();

        if let Some(area) = net.get_area_mut(&area_id) {
          Ok(area.get_map_mut().get_tile(x, y, z).gid)
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
