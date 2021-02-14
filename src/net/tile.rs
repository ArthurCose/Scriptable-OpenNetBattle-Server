#[derive(PartialEq, Eq, Default)]
pub struct Tile {
  pub gid: u32,
  pub flipped_horizontally: bool,
  pub flipped_vertically: bool,
  pub rotated: bool,
}
