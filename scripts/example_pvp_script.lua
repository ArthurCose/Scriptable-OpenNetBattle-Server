local requests = {}
local questioned_requests = {}

function handle_player_connect(player_id)
  requests[player_id] = {}
end

function handle_player_disconnect(player_id)
  requests[player_id] = nil
  questioned_requests[player_id] = nil
end

function handle_actor_interaction(player_id, other_id, button)
  if button ~= 0 then return end

  if requests[other_id] == nil then
    -- other_id is not a player id, since they're not registered in the request list
    return
  end

  local question

  if requests[player_id][other_id] then
    -- we're responding to the other player's request
    question = "Accept request?"
  else
    -- we're making a request for the other player

    if questioned_requests[player_id] then
      -- we've been asked if we want to request a fight
      -- but have not yet answered
      -- return here to prevent question spam due to interact spam
      return
    end

    local request_status = requests[other_id][player_id]

    if request_status then
      question = "Request again?"
    else
      question = "Request a battle?"
    end
  end

  local mugshot = Net.get_player_mugshot(player_id)

  Net.question_player(
    player_id,
    question,
    mugshot.texture_path,
    mugshot.animation_path
  )

  questioned_requests[player_id] = other_id
end

function handle_textbox_response(player_id, response)
  if response == 0 then
    -- response was no, no action needs to be taken
    questioned_requests[player_id] = nil
    return
  end

  local other_id = questioned_requests[player_id]

  if not requests[other_id] then
    -- the other player disconnected, we were too slow
    return
  end

  if requests[player_id][other_id] then
    -- we're saying yes to the other player's request
    requests[player_id][other_id] = nil
    Net.initiate_pvp(player_id, other_id)
  else
    -- we're making a request for the other player
    requests[other_id][player_id] = true
    Net.exclusive_player_emote(other_id, player_id, 7) -- sword emote
  end

  questioned_requests[player_id] = nil
end
