Async._tasks = {}

function _server_internal_tick(delta)
  local completed_indexes = {}

  for i, task in ipairs(Async._tasks) do
    if task(delta) then
      completed_indexes[#completed_indexes+1] = i
    end
  end

  for i = #completed_indexes, 1, -1 do
    table.remove(Async._tasks, completed_indexes[i])
  end

  if tick then
    tick(delta)
  end
end


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
    local value = nil

    function update()
      local output = table.pack(coroutine.resume(co))
      local ok = output[1]

      if not ok then
        -- value is an error
        printerr("runtime error: " .. tostring(value))
        return true
      end

      if coroutine.status(co) == "dead" then
        resolve(table.unpack(output, 2))
        return true
      end

      return false
    end

    Async._tasks[#Async._tasks+1] = update
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

    Async._tasks[#Async._tasks+1] = update
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

    Async._tasks[#Async._tasks+1] = update
  end)

  return promise
end

-- asyncified textboxes

Async._textbox_resolvers = {}
Async._next_textbox_promise = {}

function _server_internal_textbox(player_id, response)
  if handle_textbox_response then
    handle_textbox_response(player_id, response)
  end

  local next_promise = Async._next_textbox_promise[player_id]

  if next_promise[1] == 0 then
    local resolvers = Async._textbox_resolvers[player_id]
    local resolve = table.remove(resolvers, 1)

    if resolve == nil then
      return
    end

    if #next_promise > 1 then
      table.remove(next_promise, 1)
    end

    resolve(response)
  else
    next_promise[1] = next_promise[1] - 1
  end
end

function _server_internal_disconnect(player_id)
  Async._next_textbox_promise[player_id] = nil
  Async._textbox_resolvers[player_id] = nil

  if handle_player_disconnect then
    handle_player_disconnect(player_id)
  end
end

function _server_internal_request(player_id, data)
  Async._next_textbox_promise[player_id] = {0}
  Async._textbox_resolvers[player_id] = {}

  if handle_player_request then
    handle_player_request(player_id, data)
  end
end

local function create_textbox_api(function_name)
  local delegate_name = "Net._" .. function_name

  Async[function_name] = function (player_id, ...)
    local resolvers = Async._textbox_resolvers[player_id]

    if resolvers == nil then
      -- player has disconnected or never existed
      return Async.create_promise(function(resolve) resolve(nil) end)
    end

    Net._delegate(delegate_name, player_id, ...)

    return Async.create_promise(function(resolve)
      local next_promise = Async._next_textbox_promise[player_id]

      resolvers[#resolvers + 1] = resolve
      next_promise[#resolvers + 1] = 0
    end)
  end

  Net[function_name] = function (player_id, ...)
    local next_promise = Async._next_textbox_promise[player_id]

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

-- async iterators

Net.EventEmitter = {}

function Net.EventEmitter.new()
  local emitter = {
    _listeners = {},
    _any_listeners = {},
    _pending_removal = {},
    _any_pending_removal = nil,
    _destroy_listeners = {}
  }

  setmetatable(emitter, Net.EventEmitter)
  Net.EventEmitter.__index = Net.EventEmitter
  return emitter
end

function Net.EventEmitter:emit(name, ...)
  local listeners = self._listeners[name]

  -- clean up dead listeners
  local pending_removal = self._pending_removal[name]

  if pending_removal then
    for _, dead_listener in ipairs(pending_removal) do
      -- find and remove this listener
      for i, listener in ipairs(listeners) do
        if listener == dead_listener then
          table.remove(listeners, i)
          break
        end
      end
    end

    self._pending_removal[name] = nil
  end

  if self._any_pending_removal then
    for _, dead_listener in ipairs(self._any_pending_removal) do
      -- find and remove this listener
      for i, listener in ipairs(self._any_listeners) do
        if listener == dead_listener then
          table.remove(self._any_listeners, i)
          break
        end
      end
    end

    self._any_pending_removal = nil
  end

  -- call listeners
  if listeners then
    for _, listener in ipairs(listeners) do
      listener(...)
    end
  end

  for _, listener in ipairs(self._any_listeners) do
    listener(name, ...)
  end
end

function Net.EventEmitter:on(name, callback)
  local listeners = self._listeners[name]

  if listeners then
    listeners[#listeners+1] = callback
  else
    self._listeners[name] = { callback }
  end
end

function Net.EventEmitter:once(name, callback)
  local cleanup

  cleanup = function()
    self:remove_listener(name, callback)
    self:remove_listener(name, cleanup)
  end

  self:on(name, callback)
  self:on(name, cleanup)
end

function Net.EventEmitter:on_any(callback)
  local listeners = self._any_listeners

  listeners[#listeners+1] = callback
end

function Net.EventEmitter:on_any_once(callback)
  local cleanup

  cleanup = function()
    self:remove_on_any_listener(callback)
    self:remove_on_any_listener(cleanup)
  end

  self:on_any(callback)
  self:on_any(cleanup)
end

function Net.EventEmitter:remove_listener(event_name, callback)
  local pending_removal = self._pending_removal[event_name]

  if pending_removal then
    pending_removal[#pending_removal+1] = callback
  else
    self._pending_removal[event_name] = {callback}
  end
end

function Net.EventEmitter:remove_on_any_listener(callback)
  local pending_removal = self._any_pending_removal

  if pending_removal then
    pending_removal[#pending_removal+1] = callback
  else
    self._any_pending_removal = {callback}
  end
end

function Net.EventEmitter:async_iter(name)
  local promise_queue = {}
  local latest_resolve

  promise_queue[#promise_queue+1] = Async.create_promise(function(resolve)
    latest_resolve = resolve
  end)

  self:on(name, function(...)
    local last_resolve = latest_resolve

    promise_queue[#promise_queue+1] = Async.create_promise(function(resolve)
      latest_resolve = resolve
    end)

    last_resolve(...)
  end)

  self._destroy_listeners[#self._destroy_listeners+1] = function()
    latest_resolve(nil)
  end

  return function()
    local promise = table.remove(promise_queue, 1)

    if promise then
      return promise
    else
      error("read past end, are you awaiting?")
    end
  end
end

function Net.EventEmitter:async_iter_all()
  local promise_queue = {}
  local latest_resolve

  promise_queue[#promise_queue+1] = Async.create_promise(function(resolve)
    latest_resolve = resolve
  end)

  self:on_any(function(...)
    local last_resolve = latest_resolve

    promise_queue[#promise_queue+1] = Async.create_promise(function(resolve)
      latest_resolve = resolve
    end)

    last_resolve(...)
  end)

  self._destroy_listeners[#self._destroy_listeners+1] = function()
    latest_resolve(nil)
  end

  return function()
    local promise = table.remove(promise_queue, 1)

    if promise then
      return promise
    else
      error("read past end, are you awaiting?")
    end
  end
end

function Net.EventEmitter:destroy()
  for _, listener in ipairs(self._destroy_listeners) do
    listener()
  end

  self._destroy_listeners = nil
end
