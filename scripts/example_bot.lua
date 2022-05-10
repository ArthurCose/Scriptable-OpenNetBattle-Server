local Direction = require("scripts/libs/direction")

local area_id = "default"

local bot_pos = Net.get_object_by_name(area_id, "Bot Spawn")

local bot_id = Net.create_bot({
  name = "",
  area_id = area_id,
  texture_path = "/server/assets/prog.png",
  animation_path = "/server/assets/prog.animation",
  x = bot_pos.x,
  y = bot_pos.y,
  z = bot_pos.z,
  solid = true
})

local mug_texture_path = "resources/ow/prog/prog_mug.png"
local mug_animation_path = "resources/ow/prog/prog_mug.animation"

Net:on("actor_interaction", function(event)
  local player_id = event.player_id

  Net.lock_player_input(player_id)

  local player_pos = Net.get_player_position(player_id)

  Net.set_bot_direction(bot_id, Direction.from_points(bot_pos, player_pos))

  Async.question_player(player_id, "HELLO! ARE YOU DOING WELL TODAY?", mug_texture_path, mug_animation_path)
    .and_then(function(response)
      if response == nil then
        -- player disconnected
        return
      end

      if response == 1 then
        Net.message_player(player_id, "THAT'S GREAT!", mug_texture_path, mug_animation_path);
      else
        Net.message_player(player_id, "OH NO! I HOPE YOUR DAY GETS BETTER.", mug_texture_path, mug_animation_path);
      end

      Net.unlock_player_input(player_id)
  end)
end)
