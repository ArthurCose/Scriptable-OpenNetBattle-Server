pub struct Map {
  name: String,
  width: usize,
  height: usize,
  data: Vec<Vec<String>>, // row major
  cached: bool,
  cached_string: String,
}

impl Map {
  pub fn from(text: String) -> Map {
    let mut map = Map {
      name: String::new(),
      width: 0,
      height: 0,
      data: Vec::<Vec<String>>::new(),
      cached: false,
      cached_string: String::from(""),
    };

    let lines = text.split("\n");
    let mut i = 0;

    for line in lines {
      if line.len() == 0 {
        continue;
      }

      if i == 0 {
        // name
        map.name = String::from(line);
      } else {
        let row: Vec<String> = String::from(line)
          .split(",")
          .map(|i| String::from(i))
          .collect();

        if map.width < row.len() {
          map.width = row.len();
        }

        map.data.push(row);
      }

      i += 1;
    }

    map.height = map.data.len();

    for row in &mut map.data {
      if row.len() == map.width {
        continue;
      }

      row.resize(map.width, String::from("0"));
    }

    map
  }

  pub fn get_name(&self) -> &String {
    &self.name
  }

  pub fn get_width(&self) -> usize {
    self.width
  }

  pub fn get_height(&self) -> usize {
    self.height
  }

  pub fn get_tile(&self, x: usize, y: usize) -> String {
    if self.width <= x {
      String::from("0")
    } else if self.height <= y {
      String::from("0")
    } else {
      self.data[y][x].clone()
    }
  }

  pub fn set_tile(&mut self, x: usize, y: usize, id: String) {
    if self.width <= x {
      self.width = x + 1;

      for row in &mut self.data {
        row.resize(self.width, String::from("0"));
      }
    }

    if self.height <= y {
      self.height = y + 1;

      self
        .data
        .resize(self.height, vec![String::from("0"); self.width]);
    }

    if self.data[y][x] != id {
      self.data[y][x] = id;
      self.cached = false;
    }
  }

  pub fn is_dirty(&self) -> bool {
    !self.cached
  }

  pub fn render(&mut self) -> String {
    if !self.cached {
      let mut lines = vec![
        self.name.clone(),
        self.width.to_string() + " " + &self.height.to_string(),
      ];

      let mut rows: Vec<String> = self.data.iter().map(|row| row.join(",")).collect();
      lines.append(&mut rows);

      self.cached_string = lines.join("\n");
      self.cached = true;
    }

    self.cached_string.clone()
  }
}
