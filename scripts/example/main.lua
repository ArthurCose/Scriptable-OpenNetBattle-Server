function tick(delta_time)
    print("tick(" .. delta_time .. ")")
end

function handle_player_join(player_id)
    print("handle_player_join(" .. player_id .. ")")
end

function handle_player_disconnect(player_id)
    print("handle_player_disconnect(" .. player_id .. ")")
end

function handle_player_move(player_id, x, y, z)
    print("handle_player_move(" .. player_id .. ", " .. x .. ", " .. y .. ", " .. z .. ")")
end

function handle_player_avatar_change(player_id, avatar)
    print("handle_player_avatar_change(" .. player_id .. ", " .. avatar .. ")")
end

function handle_player_emote(player_id, emote)
    print("handle_player_emote(" .. player_id .. ", " .. emote .. ")")
end

print("init")
