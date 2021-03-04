use crate::helpers::unwrap_and_parse_or_default;

pub struct MapObject {
  pub id: u32,
  pub name: String,
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub width: f32,
  pub height: f32,
  pub data: MapObjectData,
}

pub enum MapObjectData {
  Point,
  Ellipse,
  Rect,
  Polygon { points: Vec<(f32, f32)> },
  TileObject { gid: u32 },
}

impl MapObject {
  pub fn from(element: &minidom::Element) -> MapObject {
    let name = element.attr("name").unwrap_or_default().to_string();
    let id: u32 = unwrap_and_parse_or_default(element.attr("id"));
    let gid: u32 = unwrap_and_parse_or_default(element.attr("gid"));
    let x: f32 = unwrap_and_parse_or_default(element.attr("x"));
    let y: f32 = unwrap_and_parse_or_default(element.attr("y"));
    let width: f32 = unwrap_and_parse_or_default(element.attr("width"));
    let height: f32 = unwrap_and_parse_or_default(element.attr("height"));

    let data = if gid != 0 {
      MapObjectData::TileObject { gid }
    } else if element.has_child("polygon", minidom::NSChoice::Any) {
      let points_element = element
        .get_child("polygon", minidom::NSChoice::Any)
        .unwrap();
      let points_str = points_element.attr("points").unwrap_or_default();

      let points = points_str
        .split(' ')
        .map(|point_str| {
          let comma_index = point_str.find(',')?;

          Some((
            point_str[0..comma_index].parse::<f32>().unwrap_or(0.0),
            point_str[comma_index + 1..].parse::<f32>().unwrap_or(0.0),
          ))
        })
        .filter_map(|point| point)
        .collect::<Vec<(f32, f32)>>();

      MapObjectData::Polygon { points }
    } else if width == 0.0 && height == 0.0 {
      MapObjectData::Point
    } else if element.has_child("ellipse", minidom::NSChoice::Any) {
      MapObjectData::Ellipse
    } else {
      MapObjectData::Rect
    };

    MapObject {
      id,
      name,
      x,
      y,
      z: 0.0,
      width,
      height,
      data,
    }
  }

  pub fn render(&mut self) -> String {
    let name_string = if !self.name.is_empty() {
      format!(" name=\"{}\"", self.name)
    } else {
      String::default()
    };

    let mut data_string = String::new();
    let mut gid_string = String::new();

    match &self.data {
      MapObjectData::Point | MapObjectData::Rect => {}
      MapObjectData::Ellipse => {
        data_string = String::from("<ellipse/>");
      }
      MapObjectData::Polygon { points } => {
        let points_string = points
          .iter()
          .map(|point| format!("{},{}", point.0, point.1))
          .collect::<Vec<String>>()
          .join(" ");

        data_string = format!("<polygon points=\"{}\"/>", points_string);
      }
      MapObjectData::TileObject { gid } => {
        gid_string = format!(" gid=\"{}\"", gid);
      }
    }

    let dimensions_string = if self.width != 0.0 && self.height != 0.0 {
      format!(" width=\"{}\" height=\"{}\"", self.width, self.height)
    } else {
      String::default()
    };

    format!(
      "\
      <object id=\"{}\"{}{} x=\"{}\" y=\"{}\"{}>\
        {}\
      </object>\
      ",
      self.id, name_string, gid_string, self.x, self.y, dimensions_string, data_string
    )
  }
}
