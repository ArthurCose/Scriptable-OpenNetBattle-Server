local Direction = require("scripts/libs/direction")
local create_custom_bot = require('scripts/bot/create_custom_bot')

local example_area_id = "default"

local bot = create_custom_bot("test", {
  name = "",
  area_id = example_area_id,
  texture_path = "/server/assets/prog.png",
  animation_path = "/server/assets/prog.animation",
  x = 1.5,
  y = 1.5,
  z = 0,
  solid = true
})

bot.mug_texture_path = "resources/ow/prog/prog_mug.png"
bot.mug_animation_path = "resources/ow/prog/prog_mug.animation"

bot.path = {
  { x=0.5, y=0.5 },
  { x=0.5, y=2.5 },
  { x=2.5, y=2.5 },
  { x=2.5, y=0.5 }
}

function bot.on_interact(player_id)
  if bot.talking_to then
    bot.message_player(player_id, "SORRY I'M BUSY TALKING TO SOMEONE RIGHT NOW.")
    return
  end

  Net.lock_player_input(player_id)
  bot.question_player(player_id, "HELLO! ARE YOU DOING WELL TODAY?")

  bot.talking_to = player_id

  local player_pos = Net.get_player_position(player_id)
  Net.set_bot_direction(bot._id, Direction.from_points(bot, player_pos))
end

function bot.on_response(player_id, response)
  if bot.talking_to ~= player_id then
    return
  end

  if response == 1 then
    -- bot.message_player(player_id, "THAT'S GREAT!");
    bot.message_player(player_id, "IT'S NIGHT TIME!");
    Net.fade_player_camera(player_id, {r=70, g=0, b=255, a=80}, 0.5)
  else
    -- bot.message_player(player_id, "OH NO! I HOPE YOUR DAY GETS BETTER.");
    bot.message_player(player_id, "OH NO! WAKE UP IT'S DAY TIME!");
    Net.fade_player_camera(player_id, {r=0, g=0, b=0, a=0}, 0.5)
  end

  Net.unlock_player_input(player_id)

  bot.talking_to = nil
end

-- server events

function tick(delta_time)
  bot._tick(delta_time)
end

function handle_actor_interaction(player_id, other_id, button)
  if button ~= 0 then return end

  bot._handle_actor_interaction(player_id, other_id)
end

function handle_textbox_response(player_id, response)
  bot._handle_textbox_response(player_id, response)
end

function handle_player_disconnect(player_id)
  bot._handle_player_disconnect(player_id)
end

function handle_player_transfer(player_id)
  bot._handle_player_disconnect(player_id)
end
