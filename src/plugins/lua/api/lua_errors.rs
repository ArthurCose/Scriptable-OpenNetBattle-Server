pub fn create_area_error(id: &str) -> rlua::Error {
  rlua::Error::RuntimeError(format!("No area matching \"{}\" found.", id))
}

pub fn create_bot_error(id: &str) -> rlua::Error {
  rlua::Error::RuntimeError(format!("No bot matching \"{}\" found.", id))
}

pub fn create_player_error(id: &str) -> rlua::Error {
  rlua::Error::RuntimeError(format!("No player matching \"{}\" found.", id))
}

pub fn create_shop_error(player_id: &str) -> rlua::Error {
  rlua::Error::RuntimeError(format!("A shop for \"{}\" is already open.", player_id))
}
