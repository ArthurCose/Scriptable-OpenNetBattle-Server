mod actor_property_animaton;
mod area_api;
mod asset_api;
mod async_api;
mod bot_api;
mod lua_errors;
mod object_api;
mod player_api;
mod player_data_api;

use crate::net::{Net, WidgetTracker};
use std::cell::RefCell;

use crate::jobs::JobPromiseManager;
use std::collections::HashMap;

pub struct ApiContext<'lua_scope, 'a> {
  pub script_path: &'lua_scope std::path::PathBuf,
  pub net_ref: &'lua_scope RefCell<&'a mut Net>,
  pub widget_tracker_ref:
    &'lua_scope RefCell<&'a mut HashMap<String, WidgetTracker<std::path::PathBuf>>>,
  pub promise_manager_ref: &'lua_scope RefCell<&'a mut JobPromiseManager>,
}

type RustLuaFunction = dyn for<'lua> FnMut(
  &ApiContext,
  rlua::Context<'lua>,
  rlua::MultiValue<'lua>,
) -> rlua::Result<rlua::MultiValue<'lua>>;

pub struct LuaApi {
  static_function_injectors: Vec<Box<dyn Fn(&rlua::Context) -> rlua::Result<()>>>,
  dynamic_function_table_and_name_pairs: Vec<(String, String)>,
  dynamic_functions: HashMap<String, Box<RustLuaFunction>>,
  table_names: Vec<String>,
}

impl LuaApi {
  pub fn new() -> LuaApi {
    let mut lua_api = LuaApi {
      static_function_injectors: Vec::new(),
      dynamic_function_table_and_name_pairs: Vec::new(),
      dynamic_functions: HashMap::new(),
      table_names: Vec::new(),
    };

    lua_api.add_global_table("Net");
    area_api::inject_dynamic(&mut lua_api);
    asset_api::inject_dynamic(&mut lua_api);
    object_api::inject_dynamic(&mut lua_api);
    player_api::inject_dynamic(&mut lua_api);
    player_data_api::inject_dynamic(&mut lua_api);
    bot_api::inject_dynamic(&mut lua_api);

    async_api::inject_static(&mut lua_api);
    async_api::inject_dynamic(&mut lua_api);

    lua_api
  }

  pub fn add_global_table(&mut self, name: &str) {
    self.table_names.push(name.to_string());
  }

  pub(self) fn add_static_injector<F>(&mut self, injector: F)
  where
    F: 'static + Send + Fn(&rlua::Context) -> rlua::Result<()>,
  {
    self.static_function_injectors.push(Box::new(injector));
  }

  pub(self) fn add_dynamic_function<F>(&mut self, table_name: &str, function_name: &str, func: F)
  where
    F: 'static
      + for<'lua> Fn(
        &ApiContext,
        rlua::Context<'lua>,
        rlua::MultiValue<'lua>,
      ) -> rlua::Result<rlua::MultiValue<'lua>>,
  {
    let table_name = String::from(table_name);
    let function_name = String::from(function_name);
    let function_id = table_name.clone() + "." + &function_name;

    self
      .dynamic_function_table_and_name_pairs
      .push((table_name, function_name));

    self.dynamic_functions.insert(function_id, Box::new(func));
  }

  pub fn inject_static(&self, lua_ctx: &rlua::Context) -> rlua::Result<()> {
    let globals = lua_ctx.globals();

    for table_name in &self.table_names {
      globals.set(table_name.as_str(), lua_ctx.create_table()?)?;
    }

    for static_function_injector in &self.static_function_injectors {
      static_function_injector(lua_ctx)?;
    }

    for (table_name, function_name) in &self.dynamic_function_table_and_name_pairs {
      let table: rlua::Table = globals.get(table_name.as_str())?;

      let function_id = table_name.clone() + "." + &function_name;

      table.set(
        function_name.as_str(),
        lua_ctx.create_function(move |lua_ctx, values: rlua::MultiValue| {
          let globals = lua_ctx.globals();
          let net_table: rlua::Table = globals.get("Net")?;
          let func: rlua::Function = net_table.get("_delegate")?;

          let value: rlua::Value = func.call((function_id.as_str(), values))?;

          Ok(value)
        })?,
      )?;
    }

    Ok(())
  }

  pub fn inject_dynamic<'lua, F>(
    &mut self,
    lua_ctx: rlua::Context<'lua>,
    api_ctx: ApiContext,
    wrapped_fn: F,
  ) -> rlua::Result<()>
  where
    F: FnMut(rlua::Context) -> rlua::Result<()>,
  {
    let mut wrapped_fn = wrapped_fn;

    lua_ctx.scope(move |scope| -> rlua::Result<()> {
      let globals = lua_ctx.globals();
      let table: rlua::Table = globals.get("Net")?;

      table.set(
        "_delegate",
        scope.create_function_mut(
          move |lua_ctx, (function_id, params): (String, rlua::MultiValue)| {
            let func = self.dynamic_functions.get_mut(&function_id);

            if let Some(func) = func {
              func(&api_ctx, lua_ctx, params)
            } else {
              Err(rlua::Error::RuntimeError(format!(
                "Function \"{}\" does not exist",
                function_id
              )))
            }
          },
        )?,
      )?;

      wrapped_fn(lua_ctx)?;

      Ok(())
    })?;

    Ok(())
  }
}
