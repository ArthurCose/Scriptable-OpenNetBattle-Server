pub fn optional_lua_string_to_optional_str<'a>(
  optional_string: &'a Option<mlua::String>,
) -> mlua::Result<Option<&'a str>> {
  optional_string
    .as_ref()
    .map(|lua_string| lua_string.to_str())
    .transpose()
}

pub fn optional_lua_string_to_str<'a>(
  optional_string: &'a Option<mlua::String>,
) -> mlua::Result<&'a str> {
  Ok(optional_lua_string_to_optional_str(optional_string)?.unwrap_or_default())
}

pub fn lua_value_to_string(
  value: mlua::Value,
  indentation: &str,
  indentation_level: usize,
) -> String {
  let mut root_table = None;
  lua_value_to_string_internal(value, indentation, indentation_level, &mut root_table)
}

fn lua_value_to_string_internal<'lua>(
  value: mlua::Value<'lua>,
  indentation: &str,
  indentation_level: usize,
  root_table: &mut Option<mlua::Table<'lua>>,
) -> String {
  match value {
    mlua::Value::Table(table) => {
      let circular_reference = match &root_table {
        Some(root_table) => root_table.equals(table.clone()).unwrap_or_default(),
        None => {
          *root_table = Some(table.clone());
          false
        }
      };

      if circular_reference {
        return String::from("Circular Reference");
      }

      let pair_strings: Vec<String> = table
        .pairs()
        .map(|pair: mlua::Result<(mlua::Value, mlua::Value)>| {
          let (key, value) = pair.unwrap();

          format!(
            "{}[{}] = {}",
            indentation,
            lua_value_to_string_internal(key, indentation, indentation_level + 1, root_table),
            lua_value_to_string_internal(value, indentation, indentation_level + 1, root_table),
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
    mlua::Value::String(lua_string) => format!(
      // wrap with ""
      "\"{}\"",
      // escape "
      String::from_utf8_lossy(lua_string.as_bytes()).replace('"', "\"")
    ),
    mlua::Value::Number(n) => n.to_string(),
    mlua::Value::Integer(i) => i.to_string(),
    mlua::Value::Boolean(b) => b.to_string(),
    mlua::Value::Nil => String::from("nil"),
    // these will create errors
    mlua::Value::Function(_) => String::from("Function"),
    mlua::Value::Thread(_) => String::from("Thread"),
    mlua::Value::LightUserData(_) => String::from("LightUserData"),
    mlua::Value::UserData(_) => String::from("UserData"),
    mlua::Value::Error(error) => format!("\"{}\"", error),
  }
}
