local state = {}

function handle_object_interaction(player_id, object_id)
  local area_id = Net.get_player_area(player_id)

  local object = Net.get_object_by_id(area_id, object_id)
  local tileGid = object.data.gid;
  local tileset = Net.get_tileset_for_tile(area_id, tileGid)

  if tileset.path ~= "../tiles/gate.tsx" then return end

  Net.message_player(player_id, "You can only cross if you can answer correctly.")
  Net.message_player(player_id, "How many clouds are in this area?")
  Net.quiz_player(player_id, "3", "5", "2")
  state[player_id] = 0
end

function handle_player_response(player_id, response)
  local current_state = state[player_id]

  if current_state == nil then return end

  state[player_id] = current_state + 1

  if current_state ~= 2 then return end

  if response == 0 then
    Net.message_player(player_id, "Correct!\n\nGates are WIP...")
    Net.move_player(player_id, 6.5, 3.5, 0)
  else
    Net.message_player(player_id, "Incorrect.")
  end
end
