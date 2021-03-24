use crate::jobs::read_file::read_file;
use crate::jobs::web_download::web_download;
use crate::jobs::web_request::web_request;
use crate::jobs::write_file::write_file;
use crate::jobs::{JobPromise, JobPromiseManager, PromiseValue};
use crate::net::Net;
use std::cell::RefCell;

pub fn add_promise_api(lua_ctx: &rlua::Context) -> rlua::Result<()> {
  let promise_api = lua_ctx.create_table()?;

  promise_api.set("_needs_cleanup", Vec::<usize>::new())?;

  promise_api.set(
    "_create_promise",
    lua_ctx.create_function(|lua_ctx, id: usize| {
      let promise = lua_ctx.create_table()?;

      promise.set(
        "is_pending",
        lua_ctx.create_function(move |lua_ctx, _: ()| {
          let async_table: rlua::Table = lua_ctx.globals().get("Async")?;
          let is_pending_function: rlua::Function = async_table.get("_is_promise_pending")?;
          let is_pending: bool = is_pending_function.call(id)?;

          Ok(is_pending)
        })?,
      )?;

      promise.set(
        "is_ready",
        lua_ctx.create_function(move |lua_ctx, _: ()| {
          let async_table: rlua::Table = lua_ctx.globals().get("Async")?;
          let is_pending_function: rlua::Function = async_table.get("_is_promise_pending")?;
          let is_pending: bool = is_pending_function.call(id)?;

          Ok(!is_pending)
        })?,
      )?;

      promise.set(
        "get_value",
        lua_ctx.create_function(move |lua_ctx, _: ()| {
          let async_table: rlua::Table = lua_ctx.globals().get("Async")?;
          let get_value_function: rlua::Function = async_table.get("_get_promise_value")?;
          let value: rlua::Value = get_value_function.call(id)?;

          Ok(value)
        })?,
      )?;

      let promise_meta_table = lua_ctx.create_table()?;

      promise_meta_table.set(
        "__gc",
        lua_ctx.create_function(move |lua_ctx, _: ()| {
          let promise_api: rlua::Table = lua_ctx.globals().get("Promise")?;
          let mut needs_cleanup: Vec<usize> = promise_api.get("_needs_cleanup")?;

          needs_cleanup.push(id);

          promise_api.set("_needs_cleanup", needs_cleanup)?;

          Ok(())
        })?,
      )?;

      promise.set_metatable(Some(promise_meta_table));

      Ok(promise)
    })?,
  )?;

  lua_ctx.globals().set("Promise", promise_api)?;

  lua_ctx
    .load(
      // todo: move to separate file?
      "\
        function Promise.await(promise)\n\
          while promise.is_pending() do\n\
            coroutine.yield()\n\
          end\n\

          return promise.get_value()\n\
        end\n\

        function Promise.all(promises)\n\
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
}

pub fn add_async_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  promise_manager_ref: &'b RefCell<&mut JobPromiseManager>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "_is_promise_pending",
    scope.create_function(move |_, id: usize| {
      let promise_manager = promise_manager_ref.borrow();

      let mut pending = true;

      if let Some(promise) = promise_manager.get_promise(id) {
        pending = promise.is_pending();
      }

      Ok(pending)
    })?,
  )?;

  api_table.set(
    "_get_promise_value",
    scope.create_function(move |lua_ctx, id: usize| {
      let mut promise_manager = promise_manager_ref.borrow_mut();

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

      Ok(value)
    })?,
  )?;

  api_table.set(
    "request",
    scope.create_function(
      move |lua_ctx, (url, options): (String, Option<rlua::Table>)| {
        let mut net = net_ref.borrow_mut();

        let method: String;
        let body: Option<String>;
        let headers: Vec<(String, String)>;

        if let Some(options) = options {
          method = options.get("method").ok().unwrap_or_default();

          body = options.get("body").ok();

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

        let lua_promise = create_lua_promise(&lua_ctx, promise_manager_ref, promise);
        Ok(lua_promise)
      },
    )?,
  )?;

  api_table.set(
    "download",
    scope.create_function(
      move |lua_ctx, (path, url, options): (String, String, Option<rlua::Table>)| {
        let mut net = net_ref.borrow_mut();

        let method: String;
        let body: Option<String>;
        let headers: Vec<(String, String)>;

        if let Some(options) = options {
          method = options.get("method").ok().unwrap_or_default();

          body = options.get("body").ok();

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

        let lua_promise = create_lua_promise(&lua_ctx, promise_manager_ref, promise);

        Ok(lua_promise)
      },
    )?,
  )?;

  api_table.set(
    "read_file",
    scope.create_function(move |lua_ctx, path: String| {
      let mut net = net_ref.borrow_mut();

      let (job, promise) = read_file(path);
      net.add_job(job);

      let lua_promise = create_lua_promise(&lua_ctx, promise_manager_ref, promise);

      Ok(lua_promise)
    })?,
  )?;

  api_table.set(
    "write_file",
    scope.create_function(move |lua_ctx, (path, content): (String, rlua::String)| {
      let mut net = net_ref.borrow_mut();

      let (job, promise) = write_file(path, content.as_bytes());
      net.add_job(job);

      let lua_promise = create_lua_promise(&lua_ctx, promise_manager_ref, promise);

      Ok(lua_promise)
    })?,
  )?;

  Ok(())
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

  let promise_api: rlua::Table = lua_ctx.globals().get("Promise")?;
  let create_promise: rlua::Function = promise_api.get("_create_promise")?;
  let lua_promise: rlua::Table = create_promise.call(id)?;

  Ok(lua_promise)
}

fn clean_up_promises(
  lua_ctx: &rlua::Context,
  promise_manager_ref: &RefCell<&mut JobPromiseManager>,
) -> rlua::Result<()> {
  let promise_api: rlua::Table = lua_ctx.globals().get("Promise")?;

  let needs_cleanup: Vec<usize> = promise_api.get("_needs_cleanup")?;

  let mut promise_manager = promise_manager_ref.borrow_mut();

  for id in needs_cleanup {
    promise_manager.remove_promise(id);
  }

  promise_api.set("_needs_cleanup", Vec::<usize>::new())?;

  Ok(())
}
