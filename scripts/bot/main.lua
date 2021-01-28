local create_custom_bot = require('scripts/bot/create_custom_bot')

local area_id = Areas.get_default_area()
local bot = create_custom_bot("test", area_id, 4, 1.5, 1.5, 0)
bot.path = {
  { x=1.5, y=1.5 },
  { x=1.5, y=3.5 },
  { x=3.5, y=3.5 },
  { x=3.5, y=1.5 }
}

function tick(delta_time)
  bot._tick(delta_time)
end

function handle_player_conversation(player_id, other_id)
  bot._handle_player_conversation(player_id, other_id)
end

function handle_player_response(player_id, response)
  bot._handle_player_response(player_id, response)
end

function handle_player_disconnect(player_id)
  bot._handle_player_disconnect(player_id)
end
