use super::Tile;

pub struct MapLayer {
  id: u32,
  name: String,
  data: Vec<u32>, // row * width + col
  width: usize,
  height: usize,
  cached: bool,
  cached_string: String,
}

impl MapLayer {
  pub fn new(id: u32, name: String, width: usize, height: usize, data: Vec<u32>) -> MapLayer {
    MapLayer {
      id,
      name,
      data,
      width,
      height,
      cached: false,
      cached_string: String::new(),
    }
  }

  pub fn get_tile(&self, x: usize, y: usize) -> Tile {
    let raw = self.data[y * self.width + x];

    Tile {
      gid: raw << 3 >> 3, // shift off tile properties
      flipped_horizontally: (raw >> 31 & 1) == 1,
      flipped_vertically: (raw >> 30 & 1) == 1,
      rotated: (raw >> 29 & 1) == 1,
    }
  }

  pub fn set_tile(&mut self, x: usize, y: usize, tile: Tile) {
    let idx = y * self.width + x;

    let compact_tile_data = tile.gid
      | (tile.flipped_horizontally as u32) << 31
      | (tile.flipped_vertically as u32) << 30
      | (tile.rotated as u32) << 29;

    if self.data[idx] != compact_tile_data {
      self.data[idx] = compact_tile_data;
      self.cached = false;
    }
  }

  pub fn render(&mut self) -> String {
    if !self.cached {
      let csv = self
        .data
        .chunks(self.width)
        .map(|row| {
          row
            .into_iter()
            .map(|gid| gid.to_string())
            .collect::<Vec<String>>()
            .join(",")
        })
        .collect::<Vec<String>>()
        .join(",");

      self.cached_string = format!(
        "\
          <layer id=\"{}\" name=\"{}\" width=\"{}\" height=\"{}\">\
            <data encoding=\"csv\">{}</data>\
          </layer>\
        ",
        self.id, self.name, self.width, self.height, csv
      );
      self.cached = true;
    }

    self.cached_string.clone()
  }
}
