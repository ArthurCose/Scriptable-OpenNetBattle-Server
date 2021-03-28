local create_custom_bot = require('scripts/bot/create_custom_bot')

local area_id = "default"
local bot = create_custom_bot("test", "", area_id, "/server/assets/prog.png", "/server/assets/prog.animation", 1.5, 1.5, 0, true)

bot.mug_texture_path = "resources/ow/prog/prog_mug.png"
bot.mug_animation_path = "resources/ow/prog/prog_mug.animation"

bot.path = {
  { x=0.5, y=0.5 },
  { x=0.5, y=2.5 },
  { x=2.5, y=2.5 },
  { x=2.5, y=0.5 }
}

local directions = {
  "Up Left",
  "Up",
  "Up Right",
  "Right",
  "Down Right",
  "Down",
  "Down Left",
  "Left",
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
  local angle = math.atan(player_pos.y - bot.y, player_pos.x - bot.x)
  local direction_index = math.floor(angle / math.pi * 4) + 5;
  Net.set_bot_direction(bot._id, directions[direction_index])
end

function bot.on_response(player_id, response)
  if bot.talking_to ~= player_id then
    return
  end

  if response == 1 then
    bot.message_player(player_id, "THAT'S GREAT!");
  else
    bot.message_player(player_id, "OH NO! I HOPE YOUR DAY GETS BETTER.");
  end

  Net.unlock_player_input(player_id)

  bot.talking_to = nil
end

-- server events

function tick(delta_time)
  bot._tick(delta_time)
end

function handle_actor_interaction(player_id, other_id)
  bot._handle_actor_interaction(player_id, other_id)
end

function handle_player_response(player_id, response)
  bot._handle_player_response(player_id, response)
end

function handle_player_disconnect(player_id)
  bot._handle_player_disconnect(player_id)
end

function handle_player_transfer(player_id)
  bot._handle_player_disconnect(player_id)
end