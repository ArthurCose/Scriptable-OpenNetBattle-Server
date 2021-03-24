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
- PATHS ARE CASE SENSITIVE OUT OF WINDOWS, also avoid `\` as that's Windows specific

## Areas

Maps for areas are stored in `./areas`. The first area players will see is `default.tmx` (required).

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
- Copy resources/ow/tiles from the client to ./tiles (relative to server folder)
  - Server will not send assets from this folder,
    but will translate the path relative to resources/ow/maps to make use of resources on the client

Tilesets:

- Type: Based on Tileset Image (other types are not currently supported)
- Object Alignment:
  - Top - For tile objects stuck to the floor such as warps
    - Set drawing offset to 0,0
  - Bottom - For tile objects that act as a wall
- Place in a Tile Layer to tune drawing offset

Layers:

- Horizontal Offset: 0
- Vertical Offset: (number of layers below this one) \* -16

### Custom properties

Map:

- Name: string
  - Area name to display in the PersonalMenu
- Song: string
  - Path to ogg file
- Background: string
  - Supported values:
    - undernet
    - robot
    - misc
    - grave
    - weather
    - medical
    - acdc
    - virus
    - judge
    - secret
    - custom
  - Case insensitive
- Background Texture: string
  - Requires "custom" Background
  - Path to background image file
- Background Animation: string
  - Requires "custom" Background
  - Path to background .animation file
  - One animation state "BG"
    - First frame of this animation determines background repetition
  - Excluding this will use texture size for background repetition
- Background Vel X: int
  - Requires "custom" Background
- Background Vel Y: int
  - Requires "custom" Background

### Types

Types are used to denote special tiles or objects understood by the client.

Home Warp:

- Tile Objects only
- Visible in minimap
- Players will be warped home if walking into the tile this object is centered on

Position Warp:

- Tile Objects only
- Visible in minimap
- Players will be warped to the set position if walking into the tile this object is centered on
- Custom properties:
  - X: float
  - Y: float
  - Z: float
  - Direction: string
    - Left
    - Right
    - Up
    - Down
    - Up Left
    - Up Right
    - Down Left
    - Down Right

Custom Warp:

- Tile Objects only
- Visible in minimap

Board:

- Tile Objects only
- Visible in minimap

Shop:

- Tile Objects only
- Visible in minimap

Stairs:

- Tiles only
- Visible in minimap
- Allows players to walk up or down a layer
- Makes tile directly above become treated as a hole
- Custom properties:

  - Direction: string
    - Marks the direction the player will travel up
    - Up Left
    - Up Right
    - Down Left
    - Down Right

## Lua API

Commented functions are in development and require changes to the client (specified below).

### Entry functions

```Lua
function tick(delta_time)
function handle_player_connect(player_id) -- player connects to the server (transfers will change initial area)
function handle_player_join(player_id) -- player enters their first area after connecting
function handle_player_transfer(player_id) -- player changes area
function handle_object_interaction(player_id, object)
function handle_navi_interaction(player_id, navi_id) -- navi_id is a player or bot id
function handle_tile_interaction(player_id, x, y, z)
function handle_player_response(player_id, response) -- response is an index
-- function handle_battle_completion(player_id, results) -- results = { status: "won" | "loss" | "ran", rank? }

-- For the following functions:
--  default action is not taken until after execution

function handle_player_disconnect(player_id)
function handle_player_move(player_id, x, y, z)

-- For the following functions:
--  default action is not taken until after execution
--  returning true will prevent the default action

function handle_player_avatar_change(player_id, texture_path, animation_path)
function handle_player_emote(player_id, emote)
```

### Net API

Interactions with the cyberworld are performed through functions attached to a global table called `Net`. The APIs defined below are those functions categorized by what they affect.

#### Area API

```Lua
-- area_id is the filename without extension
-- ./assets/my_area.tmx would be my_area

Net.list_areas() -- area_id[]
-- Net.create_area(new_area_id)
Net.reload_area(area_id) -- unstable, blocking, may throw
Net.clone_area(area_id, new_area_id)
Net.save_area(area_id) -- unstable, blocking, may throw
Net.remove_area(area_id)
Net.get_width(area_id)
Net.get_height(area_id)
Net.get_area_name(area_id)
Net.set_area_name(area_id)
Net.get_song(area_id) -- song_path
Net.set_song(area_id, song_path)
Net.get_background_name(area_id) -- background_name
Net.set_background(area_id, background_name)
Net.get_custom_background(area_id) -- { texturePath, animationPath }
Net.get_custom_background_velocity(area_id) -- { x, y }
Net.set_custom_background(area_id, texture_path, animation_path?, vel_x?, vel_y?)
Net.get_spawn_position(area_id) -- { x, y, z }
Net.set_spawn_position(area_id, x, y, z)
Net.list_tilesets(area_id) -- tileset_path[]
Net.get_tileset(area_id, tileset_path) -- { path, firstGid }?
Net.get_tileset_for_tile(area_id, tile_gid) -- { path, firstGid }?
Net.get_tile(area_id, x, y, z) -- { gid, flippedHorizontally, flippedVertically, rotated }
Net.set_tile(area_id, x, y, z, tile_gid, flip_h?, flip_v?, rotate?)
```

#### Object API

```lua
Net.list_objects(area_id) -- object_id[]
Net.get_object_by_id(area_id, object_id) -- { id, name, type, visible, x, y, z, width, height, rotation, data, customProperties }?
Net.get_object_by_name(area_id, name) -- { id, name, type, visible, x, y, z, width, height, rotation, data, customProperties }?
Net.create_object(area_id, name, x, y, layer, width, height, rotation, data) -- object_id
Net.remove_object(area_id, object_id)
Net.set_object_name(area_id, object_id, name)
Net.set_object_type(area_id, object_id, type)
Net.set_object_custom_property(area_id, object_id, name, value)
Net.resize_object(area_id, object_id, width, height)
Net.set_object_rotation(area_id, object_id, rotation)
Net.set_object_visibility(area_id, object_id, visibility)
Net.move_object(area_id, object_id, x, y, layer)
```

#### Bot API

```lua
Net.list_bots(area_id) -- bot_id[]
Net.create_bot(bot_id, name, area_id, texture_path, animation_path, x, y, z, solid?)
Net.is_bot(bot_id)
Net.remove_bot(bot_id)
Net.get_bot_area(bot_id) -- area_id
Net.get_bot_name(bot_id) -- name
Net.set_bot_name(bot_id)
Net.get_bot_position(bot_id) -- { x, y, z }
Net.move_bot(bot_id, x, y, z)
Net.set_bot_avatar(bot_id, texture_path, animation_path)
Net.set_bot_emote(bot_id, emote_id)
Net.transfer_bot(bot_id, area_id, warp_in?, x?, y?, z?)
```

#### Player API

```lua
Net.list_players(area_id) -- player_id[]
Net.is_player(player_id)
Net.get_player_area(player_id) -- area_id
Net.get_player_name(player_id) -- name
Net.set_player_name(player_id)
Net.get_player_position(player_id) -- { x, y, z }
Net.get_player_avatar(player_id) -- { texturePath, animationPath }
Net.set_player_avatar(player_id, texture_path, animation_path)
Net.is_player_in_widget(player_id)
-- Net.is_player_battling(player_id)
Net.exclude_object_for_player(player_id, object_id)
Net.include_object_for_player(player_id, object_id)
Net.move_player_camera(player_id, x, y, z, holdTimeInSeconds?)
Net.slide_player_camera(player_id, x, y, z, durationInSeconds)
Net.unlock_player_camera(player_id)
Net.lock_player(player_id)
Net.unlock_player(player_id)
Net.move_player(player_id, x, y, z)
Net.message_player(player_id, message, mug_texture_path?, mug_animation_path?)
Net.question_player(player_id, question, mug_texture_path?, mug_animation_path?)
Net.quiz_player(player_id, option_a?, option_b?, option_c?, mug_texture_path?, mug_animation_path?)
-- Net.send_virus(player_id, data)
-- Net.initiate_pvp(player_1_id, player_2_id, data)
Net.transfer_player(player_id, area_id, warp_in?, x?, y?, z?)
-- Net.transfer_server(player_id, server)
Net.kick_player(player_id, reason)
```

#### Asset API

```Lua
Net.load_asset(server_path) -- unstable, blocking, can silently fail (stores as a 0 byte asset)
Net.has_asset(server_path)
Net.get_asset_type(server_path)
Net.get_asset_size(server_path)
```

### Async API

```Lua
promise.is_ready()
promise.is_pending()
promise.get_value()
-- Promise.await(promise) -- for coroutines
-- Promise.all(promises) -- values[] - for coroutines

Async.request(url, { method?, headers?, body? }) -- promise, value = { status, headers, body }
```

## Building

This project is built with Rust, so after installing Cargo, you can compile and run the project with `cargo run`.

If you are interested in understanding the source before making changes, check out the [achitecture document](./ARCHITECTURE.md).
