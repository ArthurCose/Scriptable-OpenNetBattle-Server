use super::lua_errors::{create_area_error, create_bot_error};
use crate::net::Navi;
use crate::net::Net;
use std::cell::RefCell;

pub fn add_bot_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "list_bots",
    scope.create_function(move |_, area_id: String| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let connected_bots = area.get_connected_bots();
        let result: Vec<String> = connected_bots.to_vec();

        Ok(result)
      } else {
        Err(create_area_error(&area_id))
      }
    })?,
  )?;

  api_table.set(
    "create_bot",
    scope.create_function(
      move |_,
            (id, name, area_id, texture_path, animation_path, x, y, z): (
        String,
        String,
        String,
        String,
        String,
        f32,
        f32,
        f32,
      )| {
        let mut net = net_ref.borrow_mut();

        if net.get_area_mut(&area_id).is_some() {
          let bot = Navi {
            id,
            name,
            area_id,
            texture_path,
            animation_path,
            x,
            y,
            z,
          };

          net.add_bot(bot);

          Ok(())
        } else {
          Err(create_area_error(&id))
        }
      },
    )?,
  )?;

  api_table.set(
    "is_bot",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow();

      Ok(net.get_bot(&id).is_some())
    })?,
  )?;

  api_table.set(
    "remove_bot",
    scope.create_function(move |_, id: String| {
      let mut net = net_ref.borrow_mut();

      net.remove_bot(&id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "get_bot_area",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow_mut();

      if let Some(bot) = net.get_bot(&id) {
        Ok(bot.area_id.clone())
      } else {
        Err(create_bot_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "get_bot_name",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow_mut();

      if let Some(bot) = net.get_bot(&id) {
        Ok(bot.name.clone())
      } else {
        Err(create_bot_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "set_bot_name",
    scope.create_function(move |_, (id, name): (String, String)| {
      let mut net = net_ref.borrow_mut();

      net.set_bot_name(&id, name);

      Ok(())
    })?,
  )?;

  api_table.set(
    "get_bot_position",
    scope.create_function(move |lua_ctx, id: String| {
      let net = net_ref.borrow();

      if let Some(bot) = net.get_bot(&id) {
        let table = lua_ctx.create_table()?;
        table.set("x", bot.x)?;
        table.set("y", bot.y)?;
        table.set("z", bot.z)?;

        Ok(table)
      } else {
        Err(create_bot_error(&id))
      }
    })?,
  )?;

  api_table.set(
    "move_bot",
    scope.create_function(move |_, (id, x, y, z): (String, f32, f32, f32)| {
      let mut net = net_ref.borrow_mut();

      net.move_bot(&id, x, y, z);

      Ok(())
    })?,
  )?;

  api_table.set(
    "set_bot_avatar",
    scope.create_function(
      move |_, (id, texture_path, animation_path): (String, String, String)| {
        let mut net = net_ref.borrow_mut();

        net.set_bot_avatar(&id, texture_path, animation_path);

        Ok(())
      },
    )?,
  )?;

  api_table.set(
    "set_bot_emote",
    scope.create_function(move |_, (id, emote_id): (String, u8)| {
      let mut net = net_ref.borrow_mut();

      net.set_bot_emote(&id, emote_id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "transfer_bot",
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

        net.transfer_bot(&id, &area_id, warp_in, x, y, z);

        Ok(())
      },
    )?,
  )?;

  Ok(())
}
