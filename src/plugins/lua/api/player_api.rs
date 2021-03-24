use super::super::super::MessageTracker;
use super::lua_errors::{create_area_error, create_player_error};
use crate::net::Net;
use std::cell::RefCell;

#[allow(clippy::type_complexity)]
pub fn add_player_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  script_dir: &'b std::path::PathBuf,
  message_tracker: &'b RefCell<&mut MessageTracker<std::path::PathBuf>>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "list_players",
    scope.create_function(move |_, area_id: String| {
      let mut net = net_ref.borrow_mut();

      if let Some(area) = net.get_area_mut(&area_id) {
        let connected_players = area.get_connected_players();
        let result: Vec<String> = connected_players.to_vec();

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

      Ok(net.get_player(&id).is_some())
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
    "get_player_name",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow_mut();

      if let Some(player) = net.get_player(&id) {
        Ok(player.name.clone())
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
    scope.create_function(move |lua_ctx, id: String| {
      let net = net_ref.borrow();

      if let Some(player) = net.get_player(&id) {
        let table = lua_ctx.create_table()?;
        table.set("texturePath", player.texture_path.clone())?;
        table.set("animationPath", player.animation_path.clone())?;

        Ok(table)
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
    "is_player_in_widget",
    scope.create_function(move |_, id: String| {
      let net = net_ref.borrow();

      let is_in_widget = net.is_player_in_widget(&id);

      Ok(is_in_widget)
    })?,
  )?;

  api_table.set(
    "exclude_object_for_player",
    scope.create_function(move |_, (id, object_id): (String, u32)| {
      let mut net = net_ref.borrow_mut();

      net.exclude_object_for_player(&id, object_id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "include_object_for_player",
    scope.create_function(move |_, (id, object_id): (String, u32)| {
      let mut net = net_ref.borrow_mut();

      net.include_object_for_player(&id, object_id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "move_player_camera",
    scope.create_function(
      move |_, (id, x, y, z, duration): (String, f32, f32, f32, Option<f32>)| {
        let mut net = net_ref.borrow_mut();

        net.move_player_camera(&id, x, y, z, duration.unwrap_or_default());

        Ok(())
      },
    )?,
  )?;

  api_table.set(
    "slide_player_camera",
    scope.create_function(
      move |_, (id, x, y, z, duration): (String, f32, f32, f32, f32)| {
        let mut net = net_ref.borrow_mut();

        net.slide_player_camera(&id, x, y, z, duration);

        Ok(())
      },
    )?,
  )?;

  api_table.set(
    "unlock_player_camera",
    scope.create_function(move |_, id: String| {
      let mut net = net_ref.borrow_mut();

      net.unlock_player_camera(&id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "lock_player_input",
    scope.create_function(move |_, id: String| {
      let mut net = net_ref.borrow_mut();

      net.lock_player_input(&id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "unlock_player_input",
    scope.create_function(move |_, id: String| {
      let mut net = net_ref.borrow_mut();

      net.unlock_player_input(&id);

      Ok(())
    })?,
  )?;

  api_table.set(
    "move_player",
    scope.create_function(move |_, (id, x, y, z): (String, f32, f32, f32)| {
      let mut net = net_ref.borrow_mut();

      net.move_player(&id, x, y, z);

      Ok(())
    })?,
  )?;

  api_table.set(
    "message_player",
    scope.create_function(
      move |_,
            (id, message, mug_texture_path, mug_animation_path): (
        String,
        String,
        Option<String>,
        Option<String>,
      )| {
        let mut net = net_ref.borrow_mut();

        message_tracker
          .borrow_mut()
          .track_message(&id, script_dir.clone());

        net.message_player(
          &id,
          &message,
          &mug_texture_path.unwrap_or_default(),
          &mug_animation_path.unwrap_or_default(),
        );

        Ok(())
      },
    )?,
  )?;

  api_table.set(
    "question_player",
    scope.create_function(
      move |_,
            (id, message, mug_texture_path, mug_animation_path): (
        String,
        String,
        Option<String>,
        Option<String>,
      )| {
        let mut net = net_ref.borrow_mut();

        message_tracker
          .borrow_mut()
          .track_message(&id, script_dir.clone());

        net.question_player(
          &id,
          &message,
          &mug_texture_path.unwrap_or_default(),
          &mug_animation_path.unwrap_or_default(),
        );

        Ok(())
      },
    )?,
  )?;

  api_table.set(
    "quiz_player",
    scope.create_function(
      move |_,
            (id, option_a, option_b, option_c, mug_texture_path, mug_animation_path): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
      )| {
        let mut net = net_ref.borrow_mut();

        message_tracker
          .borrow_mut()
          .track_message(&id, script_dir.clone());

        net.quiz_player(
          &id,
          &option_a.unwrap_or_default(),
          &option_b.unwrap_or_default(),
          &option_c.unwrap_or_default(),
          &mug_texture_path.unwrap_or_default(),
          &mug_animation_path.unwrap_or_default(),
        );

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

        if let Some(player) = net.get_player(&id) {
          x = x_option.unwrap_or(player.x);
          y = y_option.unwrap_or(player.y);
          z = z_option.unwrap_or(player.z);
        } else {
          return Err(create_player_error(&id));
        }

        net.transfer_player(&id, &area_id, warp_in, x, y, z);

        Ok(())
      },
    )?,
  )?;

  api_table.set(
    "kick_player",
    scope.create_function(move |_, (id, reason): (String, String)| {
      let mut net = net_ref.borrow_mut();

      net.kick_player(&id, &reason);

      Ok(())
    })?,
  )?;

  Ok(())
}
