use super::LuaApi;
use log::*;

pub fn inject_static(lua_api: &mut LuaApi) {
  lua_api.add_static_injector(|lua_ctx| {
    let globals = lua_ctx.globals();

    globals.set(
      "print",
      lua_ctx.create_function(|_lua_ctx, args: rlua::MultiValue| {
        info!("{}", format_args(args));
        Ok(rlua::Value::Nil)
      })?,
    )?;

    globals.set(
      "printerr",
      lua_ctx.create_function(|_lua_ctx, args: rlua::MultiValue| {
        error!("{}", format_args(args));
        Ok(rlua::Value::Nil)
      })?,
    )?;

    globals.set(
      "tostring",
      lua_ctx.create_function(|_lua_ctx, value: rlua::Value| Ok(tostring(value)))?,
    )?;

    Ok(())
  });
}

fn format_args(args: rlua::MultiValue) -> String {
  args
    .into_iter()
    .map(tostring)
    .collect::<Vec<String>>()
    .join("\t")
}

fn tostring(value: rlua::Value) -> String {
  match value {
    rlua::Value::String(lua_string) => String::from_utf8_lossy(lua_string.as_bytes()).to_string(),
    _ => super::lua_helpers::lua_value_to_string(value, "\t", 0),
  }
}
