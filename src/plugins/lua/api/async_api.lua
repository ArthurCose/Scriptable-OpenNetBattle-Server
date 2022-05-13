local tasks = {}

Net:on("tick", function(event)
  local completed_indexes = {}

  for i, task in ipairs(tasks) do
    if task(event.delta_time) then
      completed_indexes[#completed_indexes+1] = i
    end
  end

  for i = #completed_indexes, 1, -1 do
    table.remove(tasks, completed_indexes[i])
  end
end)


function Async.await(promise)
  if type(promise) == "function" then
    -- awaiting an iterator that returns promises
    return function()
      return Async.await(promise())
    end
  else
    -- awaiting a promise object
    local pending = true
    local value

    promise.and_then(function (...)
      pending = false
      value = {...}
    end)

    while pending do
      coroutine.yield()
    end

    return table.unpack(value)
  end
end

function Async.await_all(promises)
  local completed = 0
  local values = {}

  for i, promise in pairs(promises) do
    promise.and_then(function (value)
      values[i] = value
      completed = completed + 1
    end)
  end

  while completed < #promises do
    coroutine.yield()
  end

  return values
end

function Async.promisify(co)
  local promise = Async.create_promise(function (resolve)
    function update()
      local output = table.pack(coroutine.resume(co))
      local ok = output[1]

      if not ok then
        -- value is an error
        printerr("runtime error: " .. tostring(output[2]))
        return true
      end

      if coroutine.status(co) == "dead" then
        resolve(table.unpack(output, 2))
        return true
      end

      return false
    end

    tasks[#tasks+1] = update
  end)

  return promise
end

function Async.create_promise(task)
  local listeners = {}
  local resolved = false
  local value

  function resolve(...)
    resolved = true
    value = {...}

    for _, listener in ipairs(listeners) do
      local success, err = pcall(function()
        listener(table.unpack(value))
      end)

      if not success then
        printerr("runtime error: " .. tostring(err))
      end
    end
  end

  local promise = {}

  function promise.and_then(listener)
    if resolved then
      listener(table.unpack(value))
    else
      listeners[#listeners+1] = listener
    end
  end

  task(resolve)

  return promise
end

function Async.sleep(duration)
  local promise = Async.create_promise(function (resolve)
    local time = 0

    function update(delta)
      time = time + delta

      if time >= duration then
        resolve()
        return true
      end

      return false
    end

    tasks[#tasks+1] = update
  end)

  return promise
end

function Async._promise_from_id(id)
  local promise = Async.create_promise(function (resolve)
    function update()
      if not Async._is_promise_pending(id) then
        resolve(Async._get_promise_value(id))
        return true
      end

      return false
    end

    tasks[#tasks+1] = update
  end)

  return promise
end

-- asyncified

local AsyncifiedTracker = {}

function AsyncifiedTracker.new()
  local tracker = {
    resolvers = {},
    next_promise = {0},
  }

  setmetatable(tracker, AsyncifiedTracker)
  AsyncifiedTracker.__index = AsyncifiedTracker
  return tracker
end

function AsyncifiedTracker:increment_count()
  self.next_promise[#self.next_promise] = self.next_promise[#self.next_promise] + 1
end

function AsyncifiedTracker:create_promise()
  return Async.create_promise(function(resolve)
    self.resolvers[#self.resolvers + 1] = resolve
    self.next_promise[#self.resolvers + 1] = 0
  end)
end

function AsyncifiedTracker:resolve(value)
  local next_promise = self.next_promise

  if next_promise[1] == 0 then
    local resolve = table.remove(self.resolvers, 1)

    if resolve == nil then
      return
    end

    if #next_promise > 1 then
      table.remove(next_promise, 1)
    end

    resolve(value)
  else
    next_promise[1] = next_promise[1] - 1
  end
end

function AsyncifiedTracker:destroy()
  for _, resolve in ipairs(self.resolvers) do
    resolve(nil)
  end
end

-- asyncified shared

local textbox_trackers = {}
local battle_trackers = {}

Net:on("player_disconnect", function(event)
  local player_id = event.player_id

  textbox_trackers[player_id]:destroy()
  textbox_trackers[player_id] = nil
  battle_trackers[player_id]:destroy()
  battle_trackers[player_id] = nil
end)

Net:on("player_request", function(event)
  local player_id = event.player_id

  textbox_trackers[player_id] = AsyncifiedTracker.new()
  battle_trackers[player_id] = AsyncifiedTracker.new()
end)

local function create_asyncified_api(function_name, trackers)
  local delegate_name = "Net._" .. function_name

  Async[function_name] = function (player_id, ...)
    local tracker = trackers[player_id]

    if tracker == nil then
      -- player has disconnected or never existed
      return Async.create_promise(function(resolve) resolve(nil) end)
    end

    Net._delegate(delegate_name, player_id, ...)

    return tracker:create_promise()
  end

  Net[function_name] = function (player_id, ...)
    local tracker = trackers[player_id]

    if tracker == nil then
      -- player has disconnected or never existed
      return
    end

    tracker:increment_count()

    Net._delegate(delegate_name, player_id, ...)
  end
end

-- asyncified textboxes

create_asyncified_api("message_player", textbox_trackers)
create_asyncified_api("question_player", textbox_trackers)
create_asyncified_api("quiz_player", textbox_trackers)
create_asyncified_api("prompt_player", textbox_trackers)

Net:on("textbox_response", function(event)
  local player_id = event.player_id

  textbox_trackers[player_id]:resolve(event.response)
end)

-- asyncified battles

create_asyncified_api("initiate_encounter", battle_trackers)
create_asyncified_api("initiate_pvp", battle_trackers)

Net:on("battle_results", function(event)
  local player_id = event.player_id

  battle_trackers[player_id]:resolve(event)
end)

-- shops

local shop_emitters = {}

Net:on("player_request", function(event)
  shop_emitters[event.player_id] = {}
end)

Net:on("player_disconnect", function(event)
  for _, emitter in ipairs(shop_emitters[event.player_id]) do
    emitter:emit("shop_close", event)
    emitter:destroy()
  end

  shop_emitters[event.player_id] = nil
end)

function Net.open_shop(player_id, ...)
  local emitters = shop_emitters[player_id]

  if not emitters then
    -- player must have disconnected
    return
  end

  Net._delegate("Net._open_shop", player_id, ...)

  local emitter = Net.EventEmitter.new()
  emitters[#emitters+1] = emitter
  return emitter
end

Net:on("shop_purchase", function(event)
  shop_emitters[event.player_id][1]:emit("shop_purchase", event)
end)

Net:on("shop_close", function(event)
  local emitter = table.remove(shop_emitters[event.player_id], 1)
  emitter:emit("shop_close", event)
  emitter:destroy()
end)

-- bbs

local bbs_emitters = {}

Net:on("player_request", function(event)
  bbs_emitters[event.player_id] = {}
end)

Net:on("player_disconnect", function(event)
  for _, emitter in ipairs(bbs_emitters[event.player_id]) do
    emitter:emit("board_close", event)
    emitter:destroy()
  end

  bbs_emitters[event.player_id] = nil
end)

function Net.open_board(player_id, ...)
  local emitters = bbs_emitters[player_id]

  if not emitters then
    -- player must have disconnected
    return
  end

  Net.close_bbs(player_id)
  Net._delegate("Net._open_board", player_id, ...)

  local emitter = Net.EventEmitter.new()
  emitters[#emitters+1] = emitter
  return emitter
end

Net:on("post_request", function(event)
  bbs_emitters[event.player_id][1]:emit("post_request", event)
end)

Net:on("post_selection", function(event)
  bbs_emitters[event.player_id][1]:emit("post_selection", event)
end)

Net:on("board_close", function(event)
  local emitter = table.remove(bbs_emitters[event.player_id], 1)
  emitter:emit("board_close", event)
  emitter:destroy()
end)
