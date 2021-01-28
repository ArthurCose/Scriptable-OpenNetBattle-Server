# Scriptable server for [OpenNetBattle](https://github.com/TheMaverickProgrammer/OpenNetBattle).

Scripts can be created through Lua. Entry scripts are read from `scripts/*/main.lua` for script projects, and `scripts/*.lua` for single file scripts.

Support for more sources such as WASM/WASI (C++, Kotlin, etc) or JavaScript can be added by creating a Plugin Interface. For an example of how implement one, take a look at the existing LuaPluginInterface.

The Plugin Interface could also be used to build a Rust based script compiled directly into the server.

## Lua API

Commented functions are in development and require changes to the client (specified below).

### Entry functions

```Lua
function tick(delta_time)
function handle_player_connect(player_id)

-- For the following functions: internal values are not set until after execution
-- this means Players.get_player_position(id) will provide the old position of the player, etc

-- function handle_player_transfer(player_id, area_id)
function handle_player_disconnect(player_id)
function handle_player_move(player_id, x, y, z)
function handle_player_avatar_change(player_id, avatar)
function handle_player_emote(player_id, emote)
-- function handle_tile_interaction(player_id, x, y, z)
-- function handle_player_conversation(player_id, other_id)
-- function handle_player_response(player_id, response) -- response is an index
-- function handle_battle_completion(player_id, results)
```

### Net API

Interactions with the cyberworld are performed through functions attached to a global table called `Net`. The APIs defined below are those functions categorized by what they affect.

#### Area API

```Lua
Net.get_default_area()
-- Net.create_area(area_id)
Net.get_width(area_id)
Net.get_height(area_id)
Net.get_tile(area_id, x, y)
Net.set_tile(area_id, x, y, id)
```

#### Bot API

```lua
Net.list_bots(area_id)
Net.create_bot(id, area_id, avatar_id, x, y, z)
Net.is_bot(id)
Net.remove_bot(id)
Net.get_bot_area(id)
Net.get_bot_position(id)
Net.move_bot(id, x, y, z)
Net.set_bot_avatar(id, avatar_id)
Net.set_bot_emote(id, emote_id)
-- Net.transfer(id, area_id)
```

#### Player API

```lua
Net.list_players(area_id)
Net.is_player(id)
Net.get_player_area(id)
Net.get_player_position(id)
Net.get_player_avatar(id)
-- Net.lock_player(id)
-- Net.unlock_player(id)
-- Net.move_player(id, x, y, z)
-- Net.send_player_message(id, message)
-- Net.send_player_question(id, question)
-- Net.send_player_menu(id, options)
-- Net.move_player_camera(id, x, y)
-- Net.slide_camera(id, x, y)
-- Net.unlock_player_camera(id)
-- Net.send_virus(id, data)
-- Net.initiate_pvp(player_1_id, player_2_id, data)
-- Net.transfer(id, area_id)
-- Net.transfer_server(id, server)
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
  - Virus battle
  - Custom asset? (background, mugshots, tiles, navis, etc)
    - Might be implemented with multiple or different packets.
      For example, tile assets may be sent with map data in the future.
  - Transfer?
    - Send the player to a different server.
  - Reset Map
    - Remove players, map tiles, etc.
    - Allows for multiple areas with one server.
- Serverbound
  - Interaction with Navi (Conversation)
  - Interaction with Tile (Interact)
  - Menu Response (for Message, MessageQuestion, etc)
    - Allows scripting the next action (textbox, camera movement, etc).
  - Map Loaded
    - Instead of the current refresh map packet.

Ordered reliable packets will be required as well, so bots don't hang on dropped responses, and so players dont miss messages or camera requests.

### Map

- Lock player when interacting with tagged tiles + navis
  - (When map format is adjusted to have this information)
- Tagged warp tiles
  - Link to other servers or locations on the same server.
  - Should be optional so scripts can take full control.

## Building

This project is built with Rust, so after installing Cargo, you can compile and run the project with `cargo run`.
