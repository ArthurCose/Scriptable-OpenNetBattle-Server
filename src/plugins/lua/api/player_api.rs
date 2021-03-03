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
        Ok(player.navi.area_id.clone())
      } else {
        Err(create_player_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "get_player_name",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow_mut();

      if let Some(player) = net.get_player(&id) {
        Ok(player.navi.name.clone())
      } else {
        Err(create_player_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "set_player_name",
    scope.create_function(move |_, (id, name): (String, String)| {
      let mut net = net_ref.borrow_mut();

      net.set_player_name(&id, name);

      Ok(())
    })?,
  )?;

  api_table.set(
    "get_player_position",
    scope.create_function(move |lua_ctx, id: String| {
      let net = net_ref.borrow();

      if let Some(player) = net.get_player(&id) {
        let table = lua_ctx.create_table()?;
        table.set("x", player.navi.x)?;
        table.set("y", player.navi.y)?;
        table.set("z", player.navi.z)?;

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
        Ok(vec![
          player.navi.texture_path.clone(),
          player.navi.animation_path.clone(),
        ])
      } else {
        Err(create_player_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "set_player_avatar",
    scope.create_function(
      move |_, (id, texture_path, animation_path): (String, String, String)| {
        let mut net = net_ref.borrow_mut();

        net.set_player_avatar(&id, texture_path, animation_path);

        Ok(())
      },
    )?,
  )?;

  api_table.set(
    "transfer_player",
    scope.create_function(
      move |_,
            (id, area_id, warp_in_option, x_option, y_option, z_option): (
        String,
        String,
        Option<bool>,
        Option<f32>,
        Option<f32>,
        Option<f32>,
      )| {
        let mut net = net_ref.borrow_mut();
        let warp_in = warp_in_option.unwrap_or(true);
        let x;
        let y;
        let z;

        if let Some(area) = net.get_area(&area_id) {
          let spawn = area.get_map().get_spawn();

          x = x_option.unwrap_or(spawn.0);
          y = y_option.unwrap_or(spawn.1);
          z = z_option.unwrap_or(0.0);
        } else {
          return Err(create_area_error(&area_id));
        }

        net.transfer_player(&id, &area_id, warp_in, x, y, z);

        Ok(())
      },
    )?,
  )?;

  Ok(())
}
