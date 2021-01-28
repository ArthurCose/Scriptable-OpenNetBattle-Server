pub fn create_area_error(id: &String) -> rlua::Error {
  rlua::Error::RuntimeError(String::from(format!("No area matching \"{}\" found.", id)))
}

pub fn create_bot_error(id: &String) -> rlua::Error {
  rlua::Error::RuntimeError(String::from(format!("No bot matching \"{}\" found.", id)))
}

pub fn create_player_error(id: &String) -> rlua::Error {
  rlua::Error::RuntimeError(String::from(format!(
    "No player matching \"{}\" found.",
    id
  )))
}
