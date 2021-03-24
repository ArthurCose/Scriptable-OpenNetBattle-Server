#[derive(Clone, Copy, Debug)]
pub enum Direction {
  None,
  Up,
  Left,
  Down,
  Right,
  UpLeft,
  UpRight,
  DownLeft,
  DownRight,
}

impl Default for Direction {
  fn default() -> Direction {
    Direction::None
  }
}

impl Direction {
  pub fn from(direction_str: &str) -> Direction {
    match direction_str {
      "Up" => Direction::Up,
      "Left" => Direction::Left,
      "Down" => Direction::Down,
      "Right" => Direction::Right,
      "Up Left" => Direction::UpLeft,
      "Up Right" => Direction::UpRight,
      "Down Left" => Direction::DownLeft,
      "Down Right" => Direction::DownRight,
      _ => Direction::None,
    }
  }
}

impl std::string::ToString for Direction {
  fn to_string(&self) -> String {
    match self {
      Direction::None => String::from("None"),
      Direction::Up => String::from("Up"),
      Direction::Left => String::from("Left"),
      Direction::Down => String::from("Down"),
      Direction::Right => String::from("Right"),
      Direction::UpLeft => String::from("Up Left"),
      Direction::UpRight => String::from("Up Right"),
      Direction::DownLeft => String::from("Down Left"),
      Direction::DownRight => String::from("Down Right"),
    }
  }
}
