function tick(delta_time)
    print("tick(" .. delta_time .. ")")
    -- os.execute("sleep 1")
end

function handle_player_join(player_id)
    print("handle_player_join(" .. player_id .. ")")
end

function handle_player_disconnect(player_id)
    print("handle_player_disconnect(" .. player_id .. ")")
end

function handle_player_move(player_id, x, y, z)
    x = x / (62 + 2.5)
    y = y / (32 + .5)
    print("handle_player_move(" .. player_id .. ", " .. x .. ", " .. y .. ", " .. z .. ")")
    set_tile(x, y, "2");
end

function handle_player_avatar_change(player_id, avatar)
    print("handle_player_avatar_change(" .. player_id .. ", " .. avatar .. ")")
end

function handle_player_emote(player_id, emote)
    print("handle_player_emote(" .. player_id .. ", " .. emote .. ")")
end

print("init")
