use super::LuaApi;

#[allow(clippy::type_complexity)]
pub fn inject_dynamic(lua_api: &mut LuaApi) {
  lua_api.add_dynamic_function("Net", "is_player_in_widget", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    let is_in_widget = net.is_player_in_widget(player_id_str);

    lua_ctx.pack_multi(is_in_widget)
  });

  lua_api.add_dynamic_function("Net", "is_player_shopping", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let net = api_ctx.net_ref.borrow();

    let is_shopping = net.is_player_shopping(player_id_str);

    lua_ctx.pack_multi(is_shopping)
  });

  lua_api.add_dynamic_function("Net", "message_player", |api_ctx, lua_ctx, params| {
    let (player_id, message, mug_texture_path, mug_animation_path): (
      rlua::String,
      rlua::String,
      Option<String>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;
    let (player_id_str, message_str) = (player_id.to_str()?, message.to_str()?);

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(tracker) = api_ctx
      .widget_tracker_ref
      .borrow_mut()
      .get_mut(player_id_str)
    {
      tracker.track_textbox(api_ctx.script_path.clone());

      net.message_player(
        player_id_str,
        message_str,
        &mug_texture_path.unwrap_or_default(),
        &mug_animation_path.unwrap_or_default(),
      );
    }

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "question_player", |api_ctx, lua_ctx, params| {
    let (player_id, message, mug_texture_path, mug_animation_path): (
      rlua::String,
      rlua::String,
      Option<String>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;
    let (player_id_str, message_str) = (player_id.to_str()?, message.to_str()?);

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(tracker) = api_ctx
      .widget_tracker_ref
      .borrow_mut()
      .get_mut(player_id_str)
    {
      tracker.track_textbox(api_ctx.script_path.clone());

      net.question_player(
        player_id_str,
        message_str,
        &mug_texture_path.unwrap_or_default(),
        &mug_animation_path.unwrap_or_default(),
      );
    }

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "quiz_player", |api_ctx, lua_ctx, params| {
    let (player_id, option_a, option_b, option_c, mug_texture_path, mug_animation_path): (
      rlua::String,
      Option<String>,
      Option<String>,
      Option<String>,
      Option<String>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(tracker) = api_ctx
      .widget_tracker_ref
      .borrow_mut()
      .get_mut(player_id_str)
    {
      tracker.track_textbox(api_ctx.script_path.clone());

      net.quiz_player(
        player_id_str,
        &option_a.unwrap_or_default(),
        &option_b.unwrap_or_default(),
        &option_c.unwrap_or_default(),
        &mug_texture_path.unwrap_or_default(),
        &mug_animation_path.unwrap_or_default(),
      );
    }

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "prompt_player", |api_ctx, lua_ctx, params| {
    let (player_id, character_limit, message): (rlua::String, Option<u16>, Option<String>) =
      lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(tracker) = api_ctx
      .widget_tracker_ref
      .borrow_mut()
      .get_mut(player_id_str)
    {
      tracker.track_textbox(api_ctx.script_path.clone());

      net.prompt_player(player_id_str, character_limit.unwrap_or(u16::MAX), message);
    }

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "open_board", |api_ctx, lua_ctx, params| {
    use crate::net::BbsPost;

    let (player_id, name, color_table, post_tables): (
      rlua::String,
      String,
      rlua::Table,
      Vec<rlua::Table>,
    ) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    if let Some(tracker) = api_ctx
      .widget_tracker_ref
      .borrow_mut()
      .get_mut(player_id_str)
    {
      tracker.track_board(api_ctx.script_path.clone());

      let color = (
        color_table.get("r")?,
        color_table.get("g")?,
        color_table.get("b")?,
      );

      let mut posts = Vec::new();
      posts.reserve(post_tables.len());

      for post_table in post_tables {
        let read: Option<bool> = post_table.get("read")?;
        let title: Option<String> = post_table.get("title")?;
        let author: Option<String> = post_table.get("author")?;

        posts.push(BbsPost {
          id: post_table.get("id")?,
          read: read.unwrap_or_default(),
          title: title.unwrap_or_default(),
          author: author.unwrap_or_default(),
        });
      }

      net.open_board(player_id_str, name, color, posts);
    }

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "prepend_posts", |api_ctx, lua_ctx, params| {
    use crate::net::BbsPost;

    let (player_id, post_tables, reference): (rlua::String, Vec<rlua::Table>, Option<String>) =
      lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    let mut posts = Vec::new();
    posts.reserve(post_tables.len());

    for post_table in post_tables {
      let read: Option<bool> = post_table.get("read")?;
      let title: Option<String> = post_table.get("title")?;
      let author: Option<String> = post_table.get("author")?;

      posts.push(BbsPost {
        id: post_table.get("id")?,
        read: read.unwrap_or_default(),
        title: title.unwrap_or_default(),
        author: author.unwrap_or_default(),
      });
    }

    net.prepend_posts(player_id_str, reference, posts);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "append_posts", |api_ctx, lua_ctx, params| {
    use crate::net::BbsPost;

    let (player_id, post_tables, reference): (rlua::String, Vec<rlua::Table>, Option<String>) =
      lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    let mut posts = Vec::new();
    posts.reserve(post_tables.len());

    for post_table in post_tables {
      let read: Option<bool> = post_table.get("read")?;
      let title: Option<String> = post_table.get("title")?;
      let author: Option<String> = post_table.get("author")?;

      posts.push(BbsPost {
        id: post_table.get("id")?,
        read: read.unwrap_or_default(),
        title: title.unwrap_or_default(),
        author: author.unwrap_or_default(),
      });
    }

    net.append_posts(player_id_str, reference, posts);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "remove_post", |api_ctx, lua_ctx, params| {
    let (player_id, post_id): (rlua::String, rlua::String) = lua_ctx.unpack_multi(params)?;
    let (player_id_str, post_id_str) = (player_id.to_str()?, post_id.to_str()?);

    let mut net = api_ctx.net_ref.borrow_mut();

    net.remove_post(player_id_str, post_id_str);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "close_bbs", |api_ctx, lua_ctx, params| {
    let player_id: rlua::String = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    let mut net = api_ctx.net_ref.borrow_mut();

    net.close_bbs(player_id_str);

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "open_shop", |api_ctx, lua_ctx, params| {
    use super::lua_errors::create_shop_error;
    use crate::net::ShopItem;

    let (player_id, item_tables, mug_texture_path, mug_animation_path): (
      rlua::String,
      Vec<rlua::Table>,
      Option<String>,
      Option<String>,
    ) = lua_ctx.unpack_multi(params)?;
    let player_id_str = player_id.to_str()?;

    if let Some(tracker) = api_ctx
      .widget_tracker_ref
      .borrow_mut()
      .get_mut(player_id_str)
    {
      tracker.track_shop(api_ctx.script_path.clone());
      let mut net = api_ctx.net_ref.borrow_mut();

      let mut items = Vec::new();
      items.reserve(item_tables.len());

      for item_table in item_tables {
        let name: String = item_table.get("name")?;
        let description: String = item_table.get("description")?;
        let price: u32 = item_table.get("price")?;

        items.push(ShopItem {
          name,
          description,
          price,
        });
      }

      net.open_shop(
        player_id_str,
        items,
        mug_texture_path.unwrap_or_default(),
        mug_animation_path.unwrap_or_default(),
      );
    }

    lua_ctx.pack_multi(())
  });
}
