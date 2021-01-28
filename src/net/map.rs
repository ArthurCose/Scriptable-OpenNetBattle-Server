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
      cached: true,
      cached_string: text.clone(),
    };

    let lines = text.split("\n");
    let mut i = 0;

    for line in lines {
      if i == 0 {
        // name
        map.name = String::from(line);
      } else if i == 1 {
        // dimensions
        let (width, height) = parse_width_and_height(line);
        map.width = width;
        map.height = height;

        map.data.reserve(height);
      } else {
        // data row
        let v: Vec<String> = String::from(line)
          .split(",")
          .map(|i| String::from(i))
          .collect();

        map.data.push(v);
      }

      i += 1;
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
      let old_width = self.width;
      self.width = x + 1;

      for row in &mut self.data {
        // capacity check needed for "attempt to subtract with overflow" fix
        if row.capacity() < self.width {
          let capacity_difference = self.width - row.capacity();

          row.reserve(capacity_difference);
        }

        for _ in old_width..self.width {
          row.push(String::from(" "))
        }
      }
    }

    if self.height <= y {
      let old_height = self.height;
      self.height = y + 1;

      for _ in old_height..self.height {
        self.data.push(vec![String::from("0"); self.width]);
      }
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

fn parse_width_and_height(line: &str) -> (usize, usize) {
  let mut width = 0;
  let mut height = 0;

  let mut j = 0;
  for n in String::from(line).split(" ") {
    match j {
      0 => width = n.parse().unwrap_or(0),
      1 => height = n.parse().unwrap_or(0),
      _ => break,
    }
    j += 1;
  }
  (width, height)
}
