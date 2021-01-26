function handle_player_move(player_id, x, y, z)
    if x < 0 or y < 0 then return end

    if Map.get_tile(x, y) ~= "H" then
        Map.set_tile(x, y, "2");
    end
end

