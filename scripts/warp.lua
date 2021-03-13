local warps = {};
local in_warp = {}

function handle_player_disconnect(player_id)
  -- cleanup
  in_warp[player_id] = nil
end

function handle_player_move(player_id, x, y, z)
  local area_id = Net.get_player_area(player_id)

  local warp = warps[area_id]

  if
    math.floor(x) == math.floor(warp.x) and
    math.floor(y) == math.floor(warp.y) and
    math.floor(z) == math.floor(warp.z)
  then
    if in_warp[player_id] == true then
      return
    end

    local new_area

    if area_id == "default" then
      new_area = "test"
    else
      new_area = "default"
    end

    local end_warp = warps[new_area]

    Net.transfer_player(player_id, new_area, true, end_warp.x, end_warp.y, end_warp.z)

    in_warp[player_id] = true
  else
    in_warp[player_id] = false
  end
end

local areas = Net.list_areas()

for _, area_id in ipairs(areas) do
  local object_ids = Net.list_objects(area_id)

  local tileset = Net.get_tileset(area_id, "../tiles/warp.tsx")
  local warp_gid = tileset.firstGid + 1

  for _, object_id in ipairs(object_ids) do
    local object = Net.get_object_by_id(area_id, object_id)

    if object.data.gid == warp_gid then
      warps[area_id] = {
        x = object.x + object.height / 2,
        y = object.y + object.height / 2,
        z = object.z
      }
    end
  end
end
