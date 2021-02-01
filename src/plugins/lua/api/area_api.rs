use super::lua_errors::create_area_error;
use crate::net::Net;
use rlua;
use std::cell::RefCell;

pub fn add_area_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "get_default_area",
    scope.create_function(move |_, ()| {
      let net = net_ref.borrow();

      Ok(net.get_default_area_id().clone())
    })?,
  )?;

  api_table.set(
    "get_width",
    scope.create_function(move |_, area_id: String| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        Ok(area.get_map().get_width())
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
        Ok(area.get_map().get_height())
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "get_tile",
    scope.create_function(move |_, (area_id, x, y): (String, usize, usize)| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        Ok(area.get_map().get_tile(x, y))
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "set_tile",
    scope.create_function(
      move |_, (area_id, x, y, id): (String, usize, usize, String)| {
        let mut net = net_ref.borrow_mut();

        if let Some(area) = net.get_area_mut(&area_id) {
          Ok(area.get_map().set_tile(x, y, id))
        } else {
          Err(create_area_error(&area_id))
        }
      },
    )?,
  )?;

  Ok(())
}
