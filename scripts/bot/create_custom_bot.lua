local function create_custom_bot(id, initial_data)
  local bot = {
    _id = id,
    x = initial_data.x,
    y = initial_data.y,
    z = initial_data.z,
    path = {},
    _path_target_index = 1,
    talking_to = nil,
    speed = 1.2,
    size = .35,
    mug_texture_path = nil,
    mug_animation_path = nil,
    on_interact = nil,
    on_response = nil
  }

  function bot._tick(delta_time)
    if bot.talking_to ~= nil then
      return
    end

    local area_id = Net.get_bot_area(bot._id);
    local player_ids = Net.list_players(area_id)

    for i = 1, #player_ids, 1 do
      local player_pos = Net.get_player_position(player_ids[i])

      if
        math.abs(player_pos.x - bot.x) < bot.size and
        math.abs(player_pos.y - bot.y) < bot.size and
        player_pos.z == bot.z
      then
        Net.move_bot(bot._id, bot.x, bot.y, bot.z)
        return
      end
    end

    local target = bot.path[bot._path_target_index]
    local angle = math.atan(target.y - bot.y, target.x - bot.x)

    local vel_x = math.cos(angle) * bot.speed
    local vel_y = math.sin(angle) * bot.speed

    bot.x = bot.x + vel_x * delta_time
    bot.y = bot.y + vel_y * delta_time

    local distance = math.sqrt((target.x - bot.x) ^ 2 + (target.y - bot.y) ^ 2)

    Net.move_bot(bot._id, bot.x, bot.y, bot.z)

    if distance < bot.speed * delta_time then
      bot._path_target_index = bot._path_target_index % #bot.path + 1
    end
  end

  function bot.message_player(player_id, message)
    Net.message_player(player_id, message, bot.mug_texture_path, bot.mug_animation_path)
  end

  function bot.question_player(player_id, message)
    Net.question_player(player_id, message, bot.mug_texture_path, bot.mug_animation_path)
  end

  function bot.quiz_player(player_id, option_a, option_b, option_c)
    Net.quiz_player(player_id, option_a, option_b, option_c, bot.mug_texture_path, bot.mug_animation_path)
  end

  function bot._handle_actor_interaction(player_id, other_id)
    if other_id ~= bot._id then
      return
    end

    if bot.on_interact then
      bot.on_interact(player_id)
    end
  end

  function bot._handle_textbox_response(player_id, response)
    if bot.on_response then
      bot.on_response(player_id, response)
    end
  end

  function bot._handle_player_disconnect(player_id)
    if bot.talking_to == player_id then
      bot.talking_to = nil
    end
  end

  Net.create_bot(id, initial_data)

  return bot
end

return create_custom_bot
