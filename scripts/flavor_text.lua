local flavorTextMap = {
  ["/server/assets/tiles/coffee.tsx"] = "A cafe sign.\nYou feel welcomed.",
  ["/server/assets/tiles/gate.tsx"] = "The gate needs a key to get through."
}

function handle_object_interaction(player_id, object_id, button)
  if button ~= 0 then return end

  local area_id = Net.get_player_area(player_id)

  local object = Net.get_object_by_id(area_id, object_id)
  local tileGid = object.data.gid;
  local tileset = Net.get_tileset_for_tile(area_id, tileGid)

  local flavorText = flavorTextMap[tileset.path]

  if flavorText ~= nil then
    Net.message_player(player_id, flavorText)
  end
end
