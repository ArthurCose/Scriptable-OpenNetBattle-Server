use super::lua_errors::{create_area_error, create_player_error};
use crate::net::Net;
use rlua;
use std::cell::RefCell;

pub fn add_player_api<'a, 'b, 'c>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "list_players",
    scope.create_function(move |_, area_id: String| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let connected_players = area.get_connected_players();
        let result: Vec<String> = connected_players.iter().map(|id| id.clone()).collect();

        Ok(result)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "is_player",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow();

      if let Some(_) = net.get_player(&id) {
        Ok(true)
      } else {
        Ok(false)
      }
    })?,
  )?;

  api_table.set(
    "get_player_area",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow_mut();

      if let Some(player) = net.get_player(&id) {
        Ok(player.area_id.clone())
      } else {
        Err(create_player_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "get_player_position",
    scope.create_function(move |lua_ctx, id: String| {
      let net = net_ref.borrow();

      if let Some(player) = net.get_player(&id) {
        let table = lua_ctx.create_table()?;
        table.set("x", player.x)?;
        table.set("y", player.y)?;
        table.set("z", player.z)?;

        Ok(table)
      } else {
        Err(create_player_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "get_player_avatar",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow_mut();

      if let Some(player) = net.get_player(&id) {
        Ok(player.avatar_id)
      } else {
        Err(create_player_error(&id))
      }
    })?,
  )?;

  Ok(())
}
