use super::LuaAPI;
use crate::jobs::{JobPromise, JobPromiseManager, PromiseValue};
use std::cell::RefCell;

pub fn inject_static(lua_api: &mut LuaAPI) {
  lua_api.add_global_table("Async");

  lua_api.add_static_injector(|lua_ctx| {
    let globals = lua_ctx.globals();
    let async_api: rlua::Table = globals.get("Async")?;

    async_api.set("_needs_cleanup", Vec::<usize>::new())?;

    async_api.set(
      "_create_promise",
      lua_ctx.create_function(|lua_ctx, id: usize| {
        let promise = lua_ctx.create_table()?;

        promise.set(
          "is_pending",
          lua_ctx.create_function(move |lua_ctx, _: ()| {
            let async_table: rlua::Table = lua_ctx.globals().get("Async")?;
            let is_pending_function: rlua::Function = async_table.get("_is_promise_pending")?;
            let is_pending: bool = is_pending_function.call(id)?;

            lua_ctx.pack_multi(is_pending)
          })?,
        )?;

        promise.set(
          "is_ready",
          lua_ctx.create_function(move |lua_ctx, _: ()| {
            let async_table: rlua::Table = lua_ctx.globals().get("Async")?;
            let is_pending_function: rlua::Function = async_table.get("_is_promise_pending")?;
            let is_pending: bool = is_pending_function.call(id)?;

            lua_ctx.pack_multi(!is_pending)
          })?,
        )?;

        promise.set(
          "get_value",
          lua_ctx.create_function(move |lua_ctx, _: ()| {
            let async_table: rlua::Table = lua_ctx.globals().get("Async")?;
            let get_value_function: rlua::Function = async_table.get("_get_promise_value")?;
            let value: rlua::Value = get_value_function.call(id)?;

            lua_ctx.pack_multi(value)
          })?,
        )?;

        let promise_meta_table = lua_ctx.create_table()?;

        promise_meta_table.set(
          "__gc",
          lua_ctx.create_function(move |lua_ctx, _: ()| {
            let async_api: rlua::Table = lua_ctx.globals().get("Async")?;
            let mut needs_cleanup: Vec<usize> = async_api.get("_needs_cleanup")?;

            needs_cleanup.push(id);

            async_api.set("_needs_cleanup", needs_cleanup)?;

            lua_ctx.pack_multi(())
          })?,
        )?;

        promise.set_metatable(Some(promise_meta_table));

        lua_ctx.pack_multi(promise)
      })?,
    )?;

    lua_ctx
      .load(
        // todo: move to separate file?
        "\
          function Async.await(promise)\n\
            while promise.is_pending() do\n\
              coroutine.yield()\n\
            end\n\

            return promise.get_value()\n\
          end\n\

          function Async.await_all(promises)\n\
            while true do\n\
              local completed = 0\n\

              for i, promise in pairs(promises) do\n\
                if promise.is_pending() then\n
                  break\n\
                end\n\
                completed = completed + 1\n\
              end\n\

              if completed == #promises then\n\
                local values = {};\n\
                for i, promise in pairs(promises) do\n\
                  values[i] = promise.get_value()\n\
                end\n\
                return values\n\
              end\n\

              coroutine.yield()\n\
            end\n\
          end\n\
        ",
      )
      .exec()?;

    Ok(())
  });
}

pub fn inject_dynamic(lua_api: &mut LuaAPI) {
  lua_api.add_dynamic_function(
    "Async",
    "_is_promise_pending",
    move |api_ctx, lua_ctx, params| {
      let id: usize = lua_ctx.unpack_multi(params)?;
      let promise_manager = api_ctx.promise_manager_ref.borrow();

      let mut pending = true;

      if let Some(promise) = promise_manager.get_promise(id) {
        pending = promise.is_pending();
      }

      lua_ctx.pack_multi(pending)
    },
  );

  lua_api.add_dynamic_function("Async", "_get_promise_value", |api_ctx, lua_ctx, params| {
    let id: usize = lua_ctx.unpack_multi(params)?;
    let mut promise_manager = api_ctx.promise_manager_ref.borrow_mut();

    let mut value = None;

    if let Some(promise) = promise_manager.get_promise_mut(id) {
      if let Some(promise_value) = promise.get_value() {
        value = match promise_value {
          PromiseValue::HttpResponse(response_data) => {
            let table = lua_ctx.create_table()?;

            table.set("status", response_data.status)?;

            let headers_table = lua_ctx.create_table()?;

            for (key, value) in &response_data.headers {
              headers_table.set(key.as_str(), value.clone())?;
            }

            table.set("headers", headers_table)?;
            table.set("body", response_data.body)?;

            Some(rlua::Value::Table(table))
          }
          PromiseValue::Bytes(bytes) => {
            let lua_string = lua_ctx.create_string(&bytes)?;

            Some(rlua::Value::String(lua_string))
          }
          PromiseValue::Success(success) => Some(rlua::Value::Boolean(success)),
          PromiseValue::None => None,
        }
      }
    }

    lua_ctx.pack_multi(value)
  });

  lua_api.add_dynamic_function("Async", "request", |api_ctx, lua_ctx, params| {
    use crate::jobs::web_request::web_request;

    let (url, options): (String, Option<rlua::Table>) = lua_ctx.unpack_multi(params)?;

    let mut net = api_ctx.net_ref.borrow_mut();

    let method: String;
    let body: Option<Vec<u8>>;
    let headers: Vec<(String, String)>;

    if let Some(options) = options {
      method = options.get("method").ok().unwrap_or_default();

      body = options
        .get("body")
        .ok()
        .map(|lua_string: rlua::String| lua_string.as_bytes().to_vec());

      headers = options
        .get("headers")
        .ok()
        .map(|table: rlua::Table| {
          table
            .pairs()
            .filter_map(|result| {
              let (key, value): (String, String) = result.ok()?;
              Some((key, value))
            })
            .collect()
        })
        .unwrap_or_default();
    } else {
      method = String::from("get");
      body = None;
      headers = Vec::new();
    }

    let (job, promise) = web_request(url, method, headers, body);
    net.add_job(job);

    let lua_promise = create_lua_promise(&lua_ctx, api_ctx.promise_manager_ref, promise);
    lua_ctx.pack_multi(lua_promise)
  });

  lua_api.add_dynamic_function("Async", "download", |api_ctx, lua_ctx, params| {
    use crate::jobs::web_download::web_download;

    let (path, url, options): (String, String, Option<rlua::Table>) =
      lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    let method: String;
    let body: Option<Vec<u8>>;
    let headers: Vec<(String, String)>;

    if let Some(options) = options {
      method = options.get("method").ok().unwrap_or_default();

      body = options
        .get("body")
        .ok()
        .map(|lua_string: rlua::String| lua_string.as_bytes().to_vec());

      headers = options
        .get("headers")
        .ok()
        .map(|table: rlua::Table| {
          table
            .pairs()
            .filter_map(|result| {
              let (key, value): (String, String) = result.ok()?;
              Some((key, value))
            })
            .collect()
        })
        .unwrap_or_default();
    } else {
      method = String::from("get");
      body = None;
      headers = Vec::new();
    }

    let (job, promise) = web_download(path, url, method, headers, body);
    net.add_job(job);

    let lua_promise = create_lua_promise(&lua_ctx, api_ctx.promise_manager_ref, promise);

    lua_ctx.pack_multi(lua_promise)
  });

  lua_api.add_dynamic_function("Async", "read_file", |api_ctx, lua_ctx, params| {
    use crate::jobs::read_file::read_file;

    let path: String = lua_ctx.unpack_multi(params)?;
    let mut net = api_ctx.net_ref.borrow_mut();

    let (job, promise) = read_file(path);
    net.add_job(job);

    let lua_promise = create_lua_promise(&lua_ctx, api_ctx.promise_manager_ref, promise);

    lua_ctx.pack_multi(lua_promise)
  });

  lua_api.add_dynamic_function("Async", "write_file", |api_ctx, lua_ctx, params| {
    let (path, content): (String, rlua::String) = lua_ctx.unpack_multi(params)?;

    use crate::jobs::write_file::write_file;
    let mut net = api_ctx.net_ref.borrow_mut();

    let (job, promise) = write_file(path, content.as_bytes());
    net.add_job(job);

    let lua_promise = create_lua_promise(&lua_ctx, api_ctx.promise_manager_ref, promise);

    lua_ctx.pack_multi(lua_promise)
  });
}

fn create_lua_promise<'a>(
  lua_ctx: &rlua::Context<'a>,
  promise_manager_ref: &RefCell<&mut JobPromiseManager>,
  promise: JobPromise,
) -> rlua::Result<rlua::Table<'a>> {
  // clean up gc'd promises every time we create new promises
  clean_up_promises(&lua_ctx, promise_manager_ref)?;

  let mut promise_manager = promise_manager_ref.borrow_mut();
  let id = promise_manager.add_promise(promise);

  let async_api: rlua::Table = lua_ctx.globals().get("Async")?;
  let create_promise: rlua::Function = async_api.get("_create_promise")?;
  let lua_promise: rlua::Table = create_promise.call(id)?;

  Ok(lua_promise)
}

fn clean_up_promises(
  lua_ctx: &rlua::Context,
  promise_manager_ref: &RefCell<&mut JobPromiseManager>,
) -> rlua::Result<()> {
  let async_api: rlua::Table = lua_ctx.globals().get("Async")?;

  let needs_cleanup: Vec<usize> = async_api.get("_needs_cleanup")?;

  let mut promise_manager = promise_manager_ref.borrow_mut();

  for id in needs_cleanup {
    promise_manager.remove_promise(id);
  }

  async_api.set("_needs_cleanup", Vec::<usize>::new())?;

  Ok(())
}
