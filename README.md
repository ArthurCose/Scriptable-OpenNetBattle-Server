# Scriptable server for [OpenNetBattle](https://github.com/TheMaverickProgrammer/OpenNetBattle).

Scripts can be created through Lua. Entry scripts are read from `scripts/*/main.lua` for script projects, and `scripts/*.lua` for single file scripts.

Support for more sources such as WASM/WASI (C++, Kotlin, etc) or JavaScript can be added by creating a Plugin Interface. For an example of how implement one, take a look at the existing LuaPluginInterface.

The Plugin Interface could also be used to build a Rust based script compiled directly into the server.

## Lua API

Commented functions are in development and require changes to the client (specified below).

### Entry functions

```Lua
function tick(delta_time)
function handle_player_join(player_id)
function handle_player_disconnect(player_id)
function handle_player_move(player_id, x, y, z)
function handle_player_avatar_change(player_id, avatar)
function handle_player_emote(player_id, emote)
-- function handle_player_conversation(player_id, other_id)
-- function handle_player_response(player_id, response) -- response is an index
```

### Global Tables

```Lua
-- todo: make this all part of one table called Area?
Map.get_width()
Map.get_height()
Map.get_tile(x, y)
Map.set_tile(x, y, id)

Bots.list_bots()
Bots.create_bot(id, avatar_id, x, y, z)
Bots.is_bot(id)
Bots.remove_bot(id)
Bots.get_bot_position(id)
Bots.move_bot(id, x, y, z)
Bots.set_bot_avatar(id, avatar_id)
Bots.set_bot_emote(id, emote_id)

Players.list_players()
Player.is_player(id)
Players.get_player_position(id)
Players.get_player_avatar(id)
-- Players.lock_player(id)
-- Players.unlock_player(id)
-- Players.move_player(id, x, y, z)
-- Players.send_player_message(id, message)
-- Players.send_player_question(id, question)
-- Players.send_player_menu(id, options)
-- Players.move_player_camera(id, x, y)
-- Players.slide_camera(id, x, y)
-- Players.unlock_player_camera(id)
```

## Proposed Changes for OpenNetBattle Client

### Packets

- Clientbound
  - Message
  - Question
  - Menu
  - Lock/Unlock player
    - Useful for cutscenes.
  - Move/Warp player (recycle existing packets?)
  - Move camera (locks camera)
  - Slide camera (locks camera)
  - Unlock camera (focus back on player)
  - Custom asset? (background, mugshots, tiles, navis, etc)
    - Might be implemented with multiple or different packets.
      For example, tile assets may be sent with map data in the future.
- Serverbound
  - Interaction with Navi (Conversation)
  - Interaction with Tile (Interact)
  - Menu Response (for Message, MessageQuestion, etc)
    - Allows scripting the next action (textbox, camera movement, etc).
  - Map Loaded
    - Instead of the current refresh map packet

Ordered reliable packets will be required as well, so bots don't hang on dropped responses, and so players dont miss messages or camera requests.

### Map

- Lock player when interacting with tagged tiles + navis
  - (When map format is adjusted to have this information)

## Building

This project is built with Rust, so after installing Cargo, you can compile and run the project with `cargo run`.
