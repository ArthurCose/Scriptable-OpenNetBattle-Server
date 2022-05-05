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

pub fn lua_value_to_string(
  value: rlua::Value,
  indentation: &str,
  indentation_level: usize,
) -> String {
  match value {
    rlua::Value::Table(table) => {
      let pair_strings: Vec<String> = table
        .pairs()
        .map(|pair: rlua::Result<(rlua::Value, rlua::Value)>| {
          let (key, value) = pair.unwrap();

          format!(
            "{}[{}] = {}",
            indentation,
            lua_value_to_string(key, indentation, indentation_level + 1),
            lua_value_to_string(value, indentation, indentation_level + 1),
          )
        })
        .collect();

      if indentation.is_empty() {
        // {pair_strings}, `{{` and `}}` are escaped versions of `{` and `}`
        format!("{{{}}}", pair_strings.join(","))
      } else {
        let indentation_string = std::iter::repeat(indentation)
          .take(indentation_level)
          .collect::<String>();

        let separator_string = std::iter::once(",\n")
          .chain(std::iter::repeat(indentation).take(indentation_level))
          .collect::<String>();

        format!(
          "{{\n{}{}\n{}}}",
          indentation_string,
          pair_strings.join(&separator_string),
          indentation_string
        )
      }
    }
    rlua::Value::String(lua_string) => format!(
      // wrap with ""
      "\"{}\"",
      // escape "
      String::from_utf8_lossy(lua_string.as_bytes()).replace('"', "\"")
    ),
    rlua::Value::Number(n) => n.to_string(),
    rlua::Value::Integer(i) => i.to_string(),
    rlua::Value::Boolean(b) => b.to_string(),
    rlua::Value::Nil => String::from("nil"),
    // these will create errors
    rlua::Value::Function(_) => String::from("Function"),
    rlua::Value::Thread(_) => String::from("Thread"),
    rlua::Value::LightUserData(_) => String::from("LightUserData"),
    rlua::Value::UserData(_) => String::from("UserData"),
    rlua::Value::Error(_) => String::from("Error"),
  }
}
