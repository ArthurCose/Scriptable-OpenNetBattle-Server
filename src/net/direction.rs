#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

  pub fn from_offset(x: f32, y: f32) -> Direction {
    if x == 0.0 && y == 0.0 {
      return Direction::None;
    }

    let x_direction = if x < 0.0 {
      Direction::UpLeft
    } else {
      Direction::DownRight
    };

    let y_direction = if y < 0.0 {
      Direction::UpRight
    } else {
      Direction::DownLeft
    };

    // using slope to calculate direction, graph if you want to take a look
    let ratio = f32::abs(y) / f32::abs(x);

    if ratio < 1.0 / 2.0 {
      return x_direction;
    } else if ratio > 2.0 {
      return y_direction;
    }

    match (x_direction, y_direction) {
      (Direction::UpLeft, Direction::UpRight) => Direction::Up,
      (Direction::UpLeft, Direction::DownLeft) => Direction::Left,
      (Direction::DownRight, Direction::UpRight) => Direction::Right,
      (Direction::DownRight, Direction::DownLeft) => Direction::Down,
      _ => Direction::None,
    }
  }

  pub fn as_str(&self) -> &str {
    match self {
      Direction::None => "None",
      Direction::Up => "Up",
      Direction::Left => "Left",
      Direction::Down => "Down",
      Direction::Right => "Right",
      Direction::UpLeft => "Up Left",
      Direction::UpRight => "Up Right",
      Direction::DownLeft => "Down Left",
      Direction::DownRight => "Down Right",
    }
  }
}

impl std::string::ToString for Direction {
  fn to_string(&self) -> String {
    String::from(self.as_str())
  }
}
