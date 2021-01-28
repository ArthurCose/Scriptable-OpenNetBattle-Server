function handle_player_move(player_id, x, y, z)
    if x < 0 or y < 0 then return end

    local area_id = Net.get_player_area(player_id)

    if Net.get_tile(area_id, x, y) ~= "H" then
        Net.set_tile(area_id, x, y, "2");
    end
end

