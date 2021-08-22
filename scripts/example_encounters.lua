local Encounters = require("scripts/libs/encounters")

-- Setup the default area's encounters
local encounters_table = { }
encounters_table["default"] = {
  min_travel = 2, -- 2 tiles are free to walk after each encounter
  chance = .05, -- 5% chance for each movement event after the minimum travel to cause an encounter
  preload = true,
  encounters = {
    {
      asset_path = "/server/assets/basic_mob1.zip",
      weight = 0.1
    }
  }
}

Encounters.setup(encounters_table)

function handle_player_join(player_id)
  Encounters.track_player(player_id)
end

function handle_player_disconnect(player_id)
  -- Drop will forget this player entry record
  -- Also useful to stop tracking players for things like cutscenes or
  -- repel items
  Encounters.drop_player(player_id)
end

function handle_player_move(player_id, x, y, z)
  Encounters.handle_player_move(player_id, x, y, z)
end
