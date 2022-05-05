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
  local pending = true
  local value

  promise.and_then(function (v)
    pending = false
    value = v
  end)

  while pending do
    coroutine.yield()
  end

  return value
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
      local ok
      ok, value = coroutine.resume(co)

      if not ok then
        -- value is an error
        printerr("runtime error: " .. tostring(value))
        return true
      end

      if coroutine.status(co) == "dead" then
        resolve(value)
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

  function resolve(v)
    resolved = true
    value = v

    for _, listener in ipairs(listeners) do
      local success, err = pcall(function() listener(value) end)

      if not success then
        printerr("runtime error: " .. tostring(err))
      end
    end
  end

  local promise = {}

  function promise.and_then(listener)
    if resolved then
      listener(value)
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
