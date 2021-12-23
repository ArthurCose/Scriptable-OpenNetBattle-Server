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
  - `/players`
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
- Background Texture: string
  - Path to background image file
- Background Animation: string
  - Path to background .animation file
  - One animation state "BG"
    - First frame of this animation determines background repetition
  - Excluding this will use texture size for background repetition
- Background Vel X: float
- Background Vel Y: float
- Background Parallax: float
- Foreground Texture: string
  - Path to foreground image file
- Foreground Animation: string
  - Path to foreground .animation file
  - One animation state "BG"
    - First frame of this animation determines foreground repetition
  - Excluding this will use texture size for foreground repetition
- Foreground Vel X: float
- Foreground Vel Y: float
- Foreground Parallax: float

### Types

Types are used to denote special tiles or objects understood by the client.

Home Warp:

- Tile Objects only
- Visible in minimap
- Players will be warped home if they walk into the tile this object is centered on
- Custom properties:
  - Direction: string
    - Left
    - Right
    - Up
    - Down
    - Up Left
    - Up Right
    - Down Left
    - Down Right

Position Warp:

- Tile Objects only
- Visible in minimap
- Players will be warped to the set position if colliding with the warp
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

Server Warp:

- Tile Objects only
- Visible in minimap
- Players will be transferred to a different server if colliding with the warp
- Custom properties:
  - Address: string
  - Port: number
  - Data: string
    - Custom data to pass to the other server
    - Can be read through handle_player_request on the other server
    - Try to keep it short! Long data strings may get ignored

Custom Server Warp:

- Tile Objects only
- Visible in minimap
- Players will be warped out if colliding with the warp, the result of the warp can be resolved in handle_custom_warp

Custom Warp:

- Tile Objects only
- Visible in minimap
- Players will be warped out if colliding with the warp, the result of the warp can be resolved in handle_custom_warp

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

Conveyor:

- Tiles only
- Visible in minimap
- Custom properties:

  - Direction: string
    - Marks the direction the player will travel
    - Up Left
    - Up Right
    - Down Left
    - Down Right
  - Speed: number? (Tiles per second, default: 6)
  - Sound Effect: string

Ice:

- Tiles only
- Custom properties:

  - Speed: number? (Tiles per second, default: 6)
  - Sound Effect: string

Treadmill:

- Tiles only
- Custom properties:

  - Direction: string
    - Marks the direction the player will travel
    - Up Left
    - Up Right
    - Down Left
    - Down Right
  - Speed: number? (Tiles per second, default: 1.875)

## Lua API

Commented functions are in development and require changes to the client (specified below).

### Entry functions

```Lua
function tick(delta_time)
function handle_player_request(player_id, data) -- player requests connection to server (only transfers and kicks should occur here)
function handle_player_connect(player_id) -- player connects to the server (good place to setup while the player is loading)
function handle_player_join(player_id) -- player enters their first area after connecting
function handle_player_transfer(player_id) -- player changes area
function handle_custom_warp(player_id, object_id) -- player warped out by a "Custom Warp" or "Custom Server Warp"
function handle_object_interaction(player_id, object_id, button)
function handle_actor_interaction(player_id, actor_id, button) -- actor_id is a player or bot id
function handle_tile_interaction(player_id, x, y, z, button)
function handle_textbox_response(player_id, response) -- response is an index or a string
function handle_board_open(player_id)
function handle_board_close(player_id)
function handle_post_selection(player_id, post_id)
function handle_post_request(player_id) -- bbs post request for infinite scroll
function handle_shop_close(player_id)
function handle_shop_purchase(player_id, item_name)
function handle_battle_results(player_id, stats) -- stats = { health: number, score: number, time: number, ran: bool, emotion: number, turns: number, npcs: { id: String, health: number }[] }
function handle_server_message(ip, port, data)
function handle_authorization(identity, host, port, data) -- a player on another server needs to be authenticated with this server

-- For the following functions:
--  default action is not taken until after execution

function handle_player_disconnect(player_id)
function handle_player_move(player_id, x, y, z)

-- For the following functions:
--  default action is not taken on the server until after execution. for the client, the action has already been taken
--  returning true will prevent the default action

-- health, max_health, and element will be updated before this function
function handle_player_avatar_change(player_id, details) -- details = { texture_path: string, animation_path: string, name: string, element: string, max_health: number }
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
Net.update_area(area_id, map_string)
Net.clone_area(area_id, new_area_id)
Net.remove_area(area_id)
Net.map_to_string(area_id)
Net.get_width(area_id)
Net.get_height(area_id)
Net.get_layer_count(area_id)
Net.get_tile_width(area_id)
Net.get_tile_height(area_id)
Net.get_area_custom_properties(area_id)
Net.get_area_custom_property(area_id, name)
Net.set_area_custom_property(area_id, name, value)
Net.get_area_name(area_id)
Net.set_area_name(area_id)
Net.get_song(area_id) -- song_path
Net.set_song(area_id, song_path)
Net.get_background(area_id) -- { texture_path, animation_path }
Net.get_background_velocity(area_id) -- { x, y }
Net.get_background_parallax(area_id) -- number
Net.set_background(area_id, texture_path, animation_path?, vel_x?, vel_y?, parallax?)
Net.get_foreground(area_id) -- { texture_path, animation_path }
Net.get_foreground_velocity(area_id) -- { x, y }
Net.get_foreground_parallax(area_id) -- number
Net.set_foreground(area_id, texture_path, animation_path?, vel_x?, vel_y?, parallax?)
Net.get_spawn_position(area_id) -- { x, y, z }
Net.set_spawn_position(area_id, x, y, z)
Net.get_spawn_direction(area_id)
Net.set_spawn_direction(area_id, direction)
Net.list_tilesets(area_id) -- tileset_path[]
Net.get_tileset(area_id, tileset_path) -- { path, first_gid }?
Net.get_tileset_for_tile(area_id, tile_gid) -- { path, first_gid }?
Net.get_tile(area_id, x, y, z) -- { gid, flipped_horizontally, flipped_vertically, rotated }
Net.set_tile(area_id, x, y, z, tile_gid, flip_h?, flip_v?, rotate?)
Net.provide_asset(area_id, path)
Net.play_sound(area_id, path)
```

#### Object API

```lua
Net.list_objects(area_id) -- object_id[]
Net.get_object_by_id(area_id, object_id) -- { id, name, type, visible, x, y, z, width, height, rotation, data, custom_properties }?
Net.get_object_by_name(area_id, name) -- { id, name, type, visible, x, y, z, width, height, rotation, data, custom_properties }?
Net.create_object(area_id, { name?, type?, visible?, x?, y?, z?, width?, height?, rotation?, data, custom_properties? }) -- object_id
Net.remove_object(area_id, object_id)
Net.set_object_name(area_id, object_id, name)
Net.set_object_type(area_id, object_id, type)
Net.set_object_custom_property(area_id, object_id, name, value)
Net.resize_object(area_id, object_id, width, height)
Net.set_object_rotation(area_id, object_id, rotation)
Net.set_object_visibility(area_id, object_id, visibility)
Net.move_object(area_id, object_id, x, y, layer)
Net.set_object_data(area_id, object_id, data)

-- possible values for data:
{
  type = "point" | "rect" | "ellipse"
}

{
  type = "polygon" | "polyline"
  points = { x, y }[],
}

{
  type = "tile",
  gid, -- int
  flipped_horizontally?, -- bool
  flipped_vertically?, -- bool
  rotated?, -- always false
}
```

#### Bot API

```lua
Net.list_bots(area_id) -- bot_id[]
Net.create_bot(bot_id, { name?, area_id?, warp_in?, texture_path?, animation_path?, animation?, x?, y?, z?, direction?, solid? })
Net.create_bot({ name?, area_id?, warp_in?, texture_path?, animation_path?, animation?, x?, y?, z?, direction?, solid? }) -- bot_id
Net.is_bot(bot_id)
Net.remove_bot(bot_id, warp_out?)
Net.get_bot_area(bot_id) -- area_id
Net.get_bot_name(bot_id) -- name
Net.set_bot_name(bot_id, name)
Net.get_bot_direction(bot_id)
Net.set_bot_direction(bot_id, direction)
Net.animate_bot_properties(bot_id, keyframes) -- unstable
Net.get_bot_position(bot_id) -- { x, y, z }
Net.move_bot(bot_id, x, y, z)
-- Net.set_bot_solid(bot_id, solid)
Net.set_bot_avatar(bot_id, texture_path, animation_path)
Net.set_bot_emote(bot_id, emote_id, use_custom_emotes?)
Net.set_bot_minimap_color(bot_id, color) -- color = { r: 0-255, g: 0-255, b: 0-255, a?: 0-255 }
Net.animate_bot(bot_id, state_name, loop?)
Net.transfer_bot(bot_id, area_id, warp_in?, x?, y?, z?)

-- keyframes:
{
  properties: {
    property: "Animation" | "Animation Speed" | "X" | "Y" | "Z" | "ScaleX" | "ScaleY" | "Rotation" | "Direction" | "Sound Effect" | "Sound Effect Loop",
    ease?: "Linear" | "In" | "Out" | "InOut" | "Floor",
    value: number | string
  }[],
  duration: number
}[]
```

#### Player API

```lua
Net.list_players(area_id) -- player_id[]
Net.is_player(player_id)
Net.get_player_area(player_id) -- area_id
Net.get_player_ip(player_id) -- address
Net.get_player_name(player_id) -- name
Net.set_player_name(player_id)
Net.get_player_direction(player_id)
Net.get_player_position(player_id) -- { x, y, z }
Net.get_player_mugshot(player_id) -- { texture_path, animation_path }
Net.get_player_avatar(player_id) -- { texture_path, animation_path }
Net.set_player_avatar(player_id, texture_path, animation_path)
Net.set_player_emote(player_id, emote_id, use_custom_emotes?)
Net.exclusive_player_emote(player_id, emoter_id, emote_id, use_custom_emotes?)
Net.set_player_minimap_color(player_id, color) -- color = { r: 0-255, g: 0-255, b: 0-255, a?: 0-255 }
Net.animate_player(player_id, state_name, loop?)
Net.animate_player_properties(player_id, keyframes) -- unstable
Net.is_player_battling(player_id)
Net.is_player_busy(player_id)
Net.provide_asset_for_player(player_id, path)
Net.play_sound_for_player(player_id, path)
Net.exclude_object_for_player(player_id, object_id)
Net.include_object_for_player(player_id, object_id)
Net.exclude_actor_for_player(player_id, actor_id)
Net.include_actor_for_player(player_id, actor_id)
Net.move_player_camera(player_id, x, y, z, holdTimeInSeconds?)
Net.fade_player_camera(player_id, color, durationInSeconds) -- color = { r: 0-255, g: 0-255, b: 0-255, a?: 0-255 }
Net.slide_player_camera(player_id, x, y, z, durationInSeconds)
Net.shake_player_camera(player_id, strength, durationInSeconds)
Net.track_with_player_camera(player_id, actor_id?)
Net.is_player_input_locked(player_id)
Net.unlock_player_camera(player_id)
Net.lock_player_input(player_id)
Net.unlock_player_input(player_id)
Net.teleport_player(player_id, warp, x, y, z, direction?)
Net.set_mod_whitelist_for_player(player_id, whitelist_path) -- whitelist has this format: `[md5] [package_id]\n`
Net.initiate_encounter(player_id, package_path, data?) -- data is a table, read as second param in package_build for the encounter package
Net.initiate_pvp(player_1_id, player_2_id, field_script_path?)
Net.transfer_player(player_id, area_id, warp_in?, x?, y?, z?, direction?)
Net.transfer_server(player_id, address, port, warp_out?, data?) -- data = string
Net.request_authorization(player_id, address, port, data?)
Net.kick_player(player_id, reason, warp_out?)
```

#### Widget API

```lua
Net.is_player_in_widget(player_id)
Net.is_player_shopping(player_id)
Net.message_player(player_id, message, mug_texture_path?, mug_animation_path?)
Net.question_player(player_id, question, mug_texture_path?, mug_animation_path?)
Net.quiz_player(player_id, option_a?, option_b?, option_c?, mug_texture_path?, mug_animation_path?)
Net.prompt_player(player_id, character_limit?, default_text?)
Net.open_board(player_id, board_name, color, posts) -- color = { r: 0-255, g: 0-255, b: 0-255 }, posts = { id: string, read: bool?, title: string?, author: string? }[]
Net.prepend_posts(player_id, posts, post_id?) -- unstable, issues arise when multiple scripts create boards at the same time
Net.append_posts(player_id, posts, post_id?) -- unstable, issues arise when multiple scripts create boards at the same time
Net.remove_post(player_id, post_id) -- unstable, issues arise when multiple scripts create boards at the same time
Net.close_bbs(player_id)
Net.open_shop(player_id, items, mug_texture_path?, mug_animation_path?) -- items = { name: string, description: string, price: number }[]
```

#### Player Data API

```lua
Net.get_player_secret(player_id) -- the secret identifier for this player. similar to a password, do not share
Net.get_player_element(player_id) -- string
Net.get_player_health(player_id)
Net.set_player_health(player_id, health)
Net.get_player_max_health(player_id)
Net.set_player_max_health(player_id, health)
Net.get_player_emotion(player_id)
Net.set_player_emotion(player_id, emotion)
Net.get_player_money(player_id)
Net.set_player_money(player_id, money)
Net.get_player_items(player_id) -- string[]
Net.give_player_item(player_id, item_id)
Net.remove_player_item(player_id, item_id)
Net.player_has_item(player_id, item_id)

Net.create_item(item_id, { name, description })
Net.get_item_name(item_id)
Net.get_item_description(item_id)
```

#### Asset API

```Lua
Net.update_asset(server_path, content)
Net.remove_asset(server_path)
Net.has_asset(server_path)
Net.get_asset_type(server_path)
Net.get_asset_size(server_path)
```

### Async API

If you want to use IO while players are connected, you'll want to use the Async API to prevent server hiccups.
Note: paths in this section use system paths and not asset paths.

```Lua
-- promise objects returned by most async functions
promise.and_then(function(value))

Async.await(promise) -- value -- for coroutines
Async.await_all(promises) -- values[] -- for coroutines
Async.promisify(coroutine) -- promise
Async.create_promise(function(resolve)) -- promise -- resolve = function(value)
Async.request(url, { method?, headers?, body? }?) -- promise, value = { status, headers, body }?
Async.download(path, url, { method?, headers?, body? }?) -- promise, value = bool
Async.read_file(path) -- promise, value = string
Async.write_file(path, content) -- promise, value = bool
Async.poll_server(address, port) -- promise, value = { max_message_size }?
Async.message_server(address, port, data) -- you will not know if this succeeds, the other server will need to reply
Async.sleep(duration) -- promise, value = nil
```

## Building

This project is built with Rust, so after installing Cargo, you can compile and run the project with `cargo run`.

If you are interested in understanding the source before making changes, check out the [achitecture document](./ARCHITECTURE.md).
