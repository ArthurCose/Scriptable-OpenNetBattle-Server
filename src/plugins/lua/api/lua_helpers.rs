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
