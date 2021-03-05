local create_custom_bot = require('scripts/bot/create_custom_bot')

local area_id = "default"
local bot = create_custom_bot("test", "", area_id, "/server/assets/prog.png", "/server/assets/prog.animation", 1.5, 1.5, 0, true)
bot.path = {
  { x=0.5, y=0.5 },
  { x=0.5, y=2.5 },
  { x=2.5, y=2.5 },
  { x=2.5, y=0.5 }
}

function tick(delta_time)
  bot._tick(delta_time)
end

function handle_navi_interaction(player_id, other_id)
  bot._handle_player_conversation(player_id, other_id)
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