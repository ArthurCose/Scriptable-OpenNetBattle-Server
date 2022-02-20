function Net.synchronize(callback)
  Net.request_update_synchronization()

  local _, err = pcall(callback)

  if err then
    print("runtime error: " .. err)
  end

  Net.request_disable_update_synchronization()
end
