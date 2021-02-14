# Scriptable server for [OpenNetBattle](https://github.com/TheMaverickProgrammer/OpenNetBattle).

Scripts can be created through Lua. Entry scripts are read from `scripts/*/main.lua` for script projects, and `scripts/*.lua` for single file scripts.

Support for more sources such as WASM/WASI (C++, Kotlin, etc) or JavaScript can be added by creating a Plugin Interface. For an example of how implement one, take a look at the existing LuaPluginInterface.

The Plugin Interface could also be used to build a Rust based script compiled directly into the server.

## Assets

Types of assets:

- Texture (.png|.bmp)
- Audio (.ogg)
- Text

Paths

- `/server`
  - Pseudo folder that represents files in memory
  - `/assets`
    - Generated at start from files in `./assets`.
    - `./assets/prog.png` can be referenced with `/server/assets/prog.png`
  - `/navis`
    - Stores avatar files sent from players (5 MiB limit)
    - Textures are stored as `[id].texture`, and animations are stored as `[id].animation`
  - `/maps`
    - Generated from areas and updated every tick.
    - Stored as `[area id].txt`
- Paths not starting with `/server` are treated as paths to client files. Files of interest are available in `resources/`

## Areas

Maps for areas are stored in `./areas`. The first area a players will see is `default.tmx` (required).

### Suggested Settings

Editor:

- Fine grid divisions: 2 (Edit -> Preferences -> Interface)
- Snap To Fine Grid (View -> Snapping)
  - When working with Object Layer
- Snap To Pixels (View -> Snapping)
  - When working with Collision shapes

Map:

- Tile Width: 64
- Tile Height: 32
- Tile Layer Format: CSV (required)
- Create map in assets
- Copy resources/ow/tiles as ./tiles (relative to server folder)
  - Server will not send assets from this folder,
    but will translate the path relative to resources/ow/maps to make use of resources on the client

Tilesets:

- Type: Based on Tileset Image (other types are not currently supported)
- Object Alignment:
  - Top - For tile objects stuck to the floor such as warps
    - Set drawing offset to 0,0
  - Bottom - For tile objects that act as a wall
- Place in a Tile Layer to tune drawing offset

### Custom properties

Map:

- Background: string
  - Path to Background
- Background Animation: string
  - Path to Background .animation file
- Background Vel X: int
- Background Vel Y: int
- Song: string
  - Path to ogg file
- Name: string

Tiles:

- Solid: bool
  - Object Layer Only
  - Defines whether the collision is used for blocking movement or just interactions

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
function handle_player_avatar_change(player_id, texture_path, animation_path)
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
Net.get_tile_gid(area_id, x, y, z)
Net.set_tile(area_id, x, y, z, gid, flip_h?, flip_v?, rotate?)
```

#### Bot API

```lua
Net.list_bots(area_id)
Net.create_bot(id, name, area_id, texture_path, animation_path, x, y, z)
Net.is_bot(id)
Net.remove_bot(id)
Net.get_bot_area(id)
Net.get_bot_name(id)
Net.set_bot_name(id)
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
Net.get_player_name(id)
Net.set_player_name(id)
Net.get_player_position(id)
-- Net.get_player_avatar(id)
Net.set_player_avatar(id, texture_path, animation_path)
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

### Map

- Lock player when interacting with tagged tiles + navis
  - (When map format is adjusted to have this information)
- Tagged warp tiles
  - Link to other servers or locations on the same server.
  - Should be optional so scripts can take full control.

## Building

This project is built with Rust, so after installing Cargo, you can compile and run the project with `cargo run`.
