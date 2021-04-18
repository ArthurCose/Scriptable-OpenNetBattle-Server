function Async.await(promise)
  while promise.is_pending() do
    coroutine.yield()
  end

  return promise.get_value()
end

function Async.await_all(promises)
  while true do
    local completed = 0

    for i, promise in pairs(promises) do
      if promise.is_pending() then
        break
      end
      completed = completed + 1
    end

    if completed == #promises then
      local values = {};
      for i, promise in pairs(promises) do
        values[i] = promise.get_value()
      end
      return values
    end

    coroutine.yield()
  end
end

function Async.promisify(co)
  local promise = {}
  local ready = false
  local value = nil

  function update()
    if not ready then
      local _
      _, value = coroutine.resume(co)
      ready = coroutine.status(co) == "dead"
    end
  end

  function promise.is_ready()
    update()
    return ready
  end

  function promise.is_pending()
    update()
    return not ready
  end

  function promise.get_value()
    update()
    return value
  end

  return promise
end
