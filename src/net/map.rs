use super::map_layer::MapLayer;
use super::map_object::MapObject;
use super::{Asset, Tile};
use crate::helpers::unwrap_and_parse_or_default;

pub struct TilesetInfo {
  first_gid: u32,
  path: String,
}

pub struct Map {
  name: String,
  background_name: String,
  song_path: String,
  width: usize,
  height: usize,
  tile_width: f32,
  tile_height: f32,
  spawn_x: f32,
  spawn_y: f32,
  tilesets: Vec<TilesetInfo>,
  layers: Vec<MapLayer>,
  next_layer_id: u32,
  objects: Vec<MapObject>,
  next_object_id: u32,
  cached: bool,
  cached_string: String,
}

impl Map {
  pub fn from(text: String) -> Map {
    let mut map = Map {
      name: String::new(),
      background_name: String::new(),
      song_path: String::new(),
      width: 0,
      height: 0,
      tile_width: 0.0,
      tile_height: 0.0,
      spawn_x: 0.0,
      spawn_y: 0.0,
      tilesets: Vec::new(),
      layers: Vec::new(),
      next_layer_id: 0,
      objects: Vec::new(),
      next_object_id: 0,
      cached: false,
      cached_string: String::from(""),
    };

    let map_element: minidom::Element = text.parse().expect("Invalid Tiled map file");

    map.width = unwrap_and_parse_or_default(map_element.attr("width"));
    map.height = unwrap_and_parse_or_default(map_element.attr("height"));
    map.tile_width = unwrap_and_parse_or_default(map_element.attr("tilewidth"));
    map.tile_height = unwrap_and_parse_or_default(map_element.attr("tileheight"));

    map.next_layer_id = unwrap_and_parse_or_default(map_element.attr("nextlayerid"));
    map.next_object_id = unwrap_and_parse_or_default(map_element.attr("nextobjectid"));

    let scale_x = 1.0 / (map.tile_width / 2.0);
    let scale_y = 1.0 / map.tile_height;

    let mut object_layers = 0;

    for child in map_element.children() {
      match child.name() {
        "properties" => {
          for property in child.children() {
            let name = property.attr("name").unwrap_or_default();
            let value = property.attr("value").unwrap_or_default().to_string();

            match name {
              "Name" => {
                map.name = value;
              }
              "Background" => {
                map.background_name = value;
              }
              "Song" => {
                map.song_path = value;
              }
              _ => {}
            }
          }
        }
        "tileset" => {
          let first_gid: u32 = unwrap_and_parse_or_default(child.attr("firstgid"));
          let mut path = child.attr("source").unwrap_or_default().to_string();

          const ASSETS_RELATIVE_PATH: &str = "../assets/";

          if path.starts_with(ASSETS_RELATIVE_PATH) {
            path = String::from("/server/assets/") + &path[ASSETS_RELATIVE_PATH.len()..];
          }

          map.tilesets.push(TilesetInfo { first_gid, path });
        }
        "layer" => {
          let id: u32 = unwrap_and_parse_or_default(child.attr("id"));
          let name: String = child.attr("name").unwrap_or_default().to_string();

          let data: Vec<u32> = child
            .get_child("data", minidom::NSChoice::Any)
            .unwrap()
            .text()
            .split(',')
            .map(|value| value.trim().parse().unwrap_or_default())
            .collect();

          map
            .layers
            .push(MapLayer::new(id, name, map.width, map.height, data));
        }
        "objectgroup" => {
          for object_element in child.children() {
            let mut map_object = MapObject::from(object_element, scale_x, scale_y);
            map_object.z = object_layers as f32;

            if map_object.name == "Home Warp" {
              map.spawn_x = map_object.x;
              map.spawn_y = map_object.y;
            }

            map.objects.push(map_object);
          }

          object_layers += 1;
        }
        _ => {}
      }
    }

    map.layers.reverse();

    map
  }

  pub fn get_tileset_paths(&self) -> impl std::iter::Iterator<Item = &String> {
    self.tilesets.iter().map(|tileset_info| &tileset_info.path)
  }

  #[allow(dead_code)]
  pub fn get_name(&self) -> &String {
    &self.name
  }

  pub fn get_width(&self) -> usize {
    self.width
  }

  pub fn get_height(&self) -> usize {
    self.height
  }

  pub fn get_spawn(&self) -> (f32, f32) {
    (self.spawn_x, self.spawn_y)
  }

  pub fn get_tile(&self, x: usize, y: usize, z: usize) -> Tile {
    if self.layers.len() <= z || self.width <= x || self.height <= y {
      Tile::default()
    } else {
      self.layers[z].get_tile(x, y)
    }
  }

  pub fn set_tile(&mut self, x: usize, y: usize, z: usize, tile: Tile) {
    // todo: expand world instead of rejecting
    if self.width <= x || self.height <= y || self.layers.len() <= z {
      return;
    }
    let layer: &mut MapLayer = &mut self.layers[z];

    if layer.get_tile(x, y) != tile {
      layer.set_tile(x, y, tile);
      self.cached = false;
    }
  }

  pub fn is_dirty(&self) -> bool {
    !self.cached
  }

  pub fn render(&mut self) -> String {
    if !self.cached {
      let mut text = vec![format!(
        "\
        <?xml version=\"1.0\" encoding=\"UTF-8\"?>\
          <map version=\"1.4\" tiledversion=\"1.4.1\" orientation=\"isometric\" \
               renderorder=\"right-down\" compressionlevel=\"0\" \
               width=\"{}\" height=\"{}\" tilewidth=\"{}\" tileheight=\"{}\" \
               infinite=\"0\" nextlayerid=\"{}\" nextobjectid=\"{}\">\
            <properties>\
              <property name=\"Name\" value=\"{}\"/>\
              <property name=\"Background\" value=\"{}\"/>\
              <property name=\"Song\" value=\"{}\"/>\
            </properties>\
        ",
        self.width,
        self.height,
        self.tile_width,
        self.tile_height,
        self.next_layer_id,
        self.next_object_id,
        self.name,
        self.background_name,
        self.song_path
      )];

      for tileset in &self.tilesets {
        text.push(format!(
          "<tileset firstgid=\"{}\" source=\"{}\"/>",
          tileset.first_gid, tileset.path
        ));
      }

      let scale_x = 1.0 / (self.tile_width / 2.0);
      let scale_y = 1.0 / self.tile_height;

      for layer_index in 0..self.layers.len() {
        text.push(self.layers[layer_index].render());

        let layer_float = layer_index as f32;

        text.push(String::from("<objectgroup>"));
        for object in &mut self.objects {
          if object.z >= layer_float && object.z < layer_float + 1.0 {
            text.push(object.render(scale_x, scale_y));
          }
        }
        text.push(String::from("</objectgroup>"));
      }

      text.push(String::from("</map>"));

      self.cached_string = text.join("");
      self.cached = true;
    }

    self.cached_string.clone()
  }

  pub fn generate_asset(&mut self) -> Asset {
    use super::AssetData;

    Asset {
      data: AssetData::Text(self.render()),
      dependencies: self
        .get_tileset_paths()
        .filter(|path| path.starts_with("/server/")) // tileset provided by server
        .cloned()
        .collect(),
    }
  }
}
