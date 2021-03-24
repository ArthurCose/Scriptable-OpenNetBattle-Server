use crate::jobs::web_request::web_request;
use crate::jobs::{JobPromiseManager, PromiseValue};
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
          let value: rlua::Table = get_value_function.call(id)?;

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

              Some(table)
            }
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
        // we should clean up gc'd promises any time we create new promises
        clean_up_promises(&lua_ctx, promise_manager_ref)?;

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

        let mut promise_manager = promise_manager_ref.borrow_mut();
        let id = promise_manager.add_promise(promise);

        let promise_api: rlua::Table = lua_ctx.globals().get("Promise")?;
        let create_promise: rlua::Function = promise_api.get("_create_promise")?;
        let promise: rlua::Table = create_promise.call(id)?;

        Ok(promise)
      },
    )?,
  )?;

  Ok(())
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
