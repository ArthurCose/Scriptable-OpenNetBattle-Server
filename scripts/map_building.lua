function handle_tile_interaction(player_id, x, y, z, button)
  if button ~= 0 or x < 0 or y < 0 then return end

  x = math.floor(x)
  y = math.floor(y)
  z = math.floor(z)

  local area_id = Net.get_player_area(player_id)
  local tile_gid = Net.get_tile(area_id, x, y, z).gid

  if tile_gid == 0 then
    Net.set_tile(area_id, x, y, z, 1)
  elseif tile_gid == 1 and not has_player(area_id, x, y, z) then
    Net.set_tile(area_id, x, y, z, 0)
  end
end

function has_player(area_id, x, y, z)
  local player_ids = Net.list_players(area_id)

  for i = 1, #player_ids, 1 do
    local player_pos = Net.get_player_position(player_ids[i])

    if
      x == math.floor(player_pos.x) and
      y == math.floor(player_pos.y) and
      z == math.floor(player_pos.z)
    then
      -- block updates to this tile
      return true
    end
  end

  return false
end
