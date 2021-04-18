Async._tasks = {}

function _server_internal_tick(delta)
  local completed_indexes = {}

  for i, task in ipairs(Async._tasks) do
    if task() then
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
      local _
      _, value = coroutine.resume(co)

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
      pcall(function() listener(value) end)
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
