pub fn optional_lua_string_to_optional_str<'a>(
  optional_string: &'a Option<rlua::String>,
) -> rlua::Result<Option<&'a str>> {
  optional_string
    .as_ref()
    .map(|lua_string| lua_string.to_str())
    .transpose()
}

pub fn optional_lua_string_to_str<'a>(
  optional_string: &'a Option<rlua::String>,
) -> rlua::Result<&'a str> {
  Ok(optional_lua_string_to_optional_str(optional_string)?.unwrap_or_default())
}

pub fn lua_value_to_string(value: rlua::Value) -> String {
  match value {
    rlua::Value::Table(table) => {
      let pair_strings: Vec<String> = table
        .pairs()
        .map(|pair: rlua::Result<(rlua::Value, rlua::Value)>| {
          let (key, value) = pair.unwrap();

          format!(
            "[{}]={}",
            lua_value_to_string(key),
            lua_value_to_string(value),
          )
        })
        .collect();

      // {pair_strings}, `{{` and `}}` are escaped versions of `{` and `}`
      format!("{{{}}}", pair_strings.join(","))
    }
    rlua::Value::String(lua_string) => format!(
      // wrap with ""
      "\"{}\"",
      // escape "
      lua_string.to_str().unwrap_or_default().replace('"', "\"")
    ),
    rlua::Value::Number(n) => n.to_string(),
    rlua::Value::Integer(i) => i.to_string(),
    rlua::Value::Boolean(b) => b.to_string(),
    _ => String::from("nil"),
  }
}
