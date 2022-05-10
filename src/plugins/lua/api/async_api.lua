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

-- asyncified textboxes

local textbox_resolvers = {}
local next_textbox_promise = {}

Net:on("textbox_response", function(event)
  local player_id = event.player_id

  local next_promise = next_textbox_promise[player_id]

  if next_promise[1] == 0 then
    local resolvers = textbox_resolvers[player_id]
    local resolve = table.remove(resolvers, 1)

    if resolve == nil then
      return
    end

    if #next_promise > 1 then
      table.remove(next_promise, 1)
    end

    resolve(event.response)
  else
    next_promise[1] = next_promise[1] - 1
  end
end)

Net:on("player_disconnect", function(event)
  local player_id = event.player_id

  next_textbox_promise[player_id] = nil
  textbox_resolvers[player_id] = nil
end)

Net:on("player_request", function(event)
  local player_id = event.player_id

  next_textbox_promise[player_id] = {0}
  textbox_resolvers[player_id] = {}
end)

local function create_textbox_api(function_name)
  local delegate_name = "Net._" .. function_name

  Async[function_name] = function (player_id, ...)
    local resolvers = textbox_resolvers[player_id]

    if resolvers == nil then
      -- player has disconnected or never existed
      return Async.create_promise(function(resolve) resolve(nil) end)
    end

    Net._delegate(delegate_name, player_id, ...)

    return Async.create_promise(function(resolve)
      local next_promise = next_textbox_promise[player_id]

      resolvers[#resolvers + 1] = resolve
      next_promise[#resolvers + 1] = 0
    end)
  end

  Net[function_name] = function (player_id, ...)
    local next_promise = next_textbox_promise[player_id]

    if next_promise == nil then
      -- player has disconnected or never existed
      return
    end

    next_promise[#next_promise] = next_promise[#next_promise] + 1

    Net._delegate(delegate_name, player_id, ...)
  end
end

create_textbox_api("message_player")
create_textbox_api("question_player")
create_textbox_api("quiz_player")
create_textbox_api("prompt_player")

-- shops

local shop_emitters = {}

Net:on("player_request", function(event)
  shop_emitters[event.player_id] = {}
end)

Net:on("player_disconnect", function(event)
  for _, emitter in ipairs(shop_emitters[event.player_id]) do
    emitter:emit("close", event)
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
  shop_emitters[event.player_id][1]:emit("purchase", event)
end)

Net:on("shop_close", function(event)
  local emitter = table.remove(shop_emitters[event.player_id], 1)
  emitter:emit("close", event)
  emitter:destroy()
end)

-- bbs

local bbs_emitters = {}

Net:on("player_request", function(event)
  bbs_emitters[event.player_id] = {}
end)

Net:on("player_disconnect", function(event)
  for _, emitter in ipairs(bbs_emitters[event.player_id]) do
    emitter:emit("close", event)
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
  bbs_emitters[event.player_id][1]:emit("request", event)
end)

Net:on("post_selection", function(event)
  bbs_emitters[event.player_id][1]:emit("selection", event)
end)

Net:on("board_close", function(event)
  local emitter = table.remove(bbs_emitters[event.player_id], 1)
  emitter:emit("close", event)
  emitter:destroy()
end)
