local states = {};

function handle_player_join(player_id)
  Net.lock_player_input(player_id)
  states[player_id] = { status = 0, remaining_time = 1.8 }
end

function tick(elapsed)
  for player_id,state in pairs(states) do
    state.remaining_time = state.remaining_time - elapsed
    -- states[player_id] = remaining_time

    if state.remaining_time > 0 then
      goto continue
    end

    if state.status == 0 then
      Net.slide_player_camera(player_id, 4.5, 3.5, 0, 3)
      Net.slide_player_camera(player_id, 7.5, 3.5, 0, 2)
      Net.move_player_camera(player_id, 7.5, 3.5, 0, 2)
      Net.unlock_player_camera(player_id)
      state.status = 1
      state.remaining_time = 7.1
    else
      Net.unlock_player_input(player_id)
      states[player_id] = nil
    end

    ::continue::
  end
end