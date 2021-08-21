local lib = { db = {} }

lib._BIG_TILE_MOVEMENT = 5 -- users can't walk/run 5 tiles a frame without help from the server

function lib:reset(player_id) 
    local pos = Net.get_player_position(player_id)
    self.db[player_id] = { last_pos = pos, amt = 0 }
end

function lib:drop(player_id)
    self.db[player_id] = nil
end

function lib:setup(encounter_table, ignore_tiles_greater_than)
    self.table = encounter_table

    for area_id, entry in pairs(self.table) do 
        local weighted_sum = 0

        for key, mob in pairs(entry.mobs) do
            weighted_sum = weighted_sum + mob.weight 
        end

        if weighted_sum > 1.0 then 
            -- we must normalize
            for key, mob in pairs(entry.mobs) do
                mob.weight = mob.weight / weighted_sum
            end
        end
    end

    if ignore_tiles_greater_than ~= nil then 
        self._BIG_TILE_MOVEMENT = ignore_tiles_greater_than
    end
end

function lib:handle_player_move(player_id, x, y, z) 
    local pos = {x = x, y = y}
    local last_pos = self.db[player_id].last_pos 
    local amtx = math.abs(last_pos.x-pos.x)
    local amty = math.abs(last_pos.y-pos.y)
    
    if amtx + amty > self._BIG_TILE_MOVEMENT then
        return
    end
    
    local total_amt = self.db[player_id].amt + amtx + amty
    self.db[player_id].amt = total_amt
    self.db[player_id].last_pos = pos
    
    local area_id = Net.get_player_area(player_id)

    local required_travel_amt = self.table[area_id].required_travel_amt
    for key, value in pairs(self.table[area_id].mobs) do
        if required_travel_amt <= total_amt then 
            local r = math.random(0, 100)

            if r <= value.weight*100.0 then
                self:reset(player_id)
                Net.initiate_mob(player_id, value.asset_path)
            end
        end
    end
end

function lib:initiate_direct_encounter(player_id, asset_path)
    self:reset(player_id)
    Net.initiate_mob(player_id, asset_path)
end

return lib

--[[
    -- Setup example below

    local encounters_table = { }
    encounters_table["central_area_1"].required_travel_amt = 10 -- 10 tiles walked
    encounters_table["central_area_1"].mobs = {
        { 
            asset_path = "/server/assets/basic_mob1.zip",
            weight = 0.1
        },
        { 
            asset_path = "/server/assets/basic_mob2.zip",
            weight = 0.5
        },
        { 
            asset_path = "/server/assets/basic_mob3.zip",
            weight = 0.01
        },
    }
    encounters_table["central_area_2"].required_travel_amt = 50 -- less enemies...
    encounters_table["central_area_2"].mobs = { ... etc ... }

    Encounters:setup(encounters_table)
]]