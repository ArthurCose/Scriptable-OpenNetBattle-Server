#[derive(Clone, Debug)]
pub struct Asset {
  pub data: AssetData,
  pub alternate_names: Vec<AssetDependency>,
  pub dependencies: Vec<AssetDependency>,
  pub last_modified: u64,
  pub cachable: bool, // allows the client to know if they can cache this asset or if it's dynamic
}

#[derive(Clone, Debug)]
pub enum AssetData {
  Text(String),
  Texture(Vec<u8>),
  Audio(Vec<u8>),
  Data(Vec<u8>),
}

#[derive(Clone, Debug)]
pub enum AssetDependency {
  AssetPath(String),
  ScriptedCharacter(String),
}

impl Asset {
  pub fn load_from_memory(path: &std::path::Path, data: &[u8]) -> Asset {
    let asset_data = resolve_asset_data(path, data);

    let last_modified = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("Current time is before epoch?")
      .as_secs();

    let mut asset = Asset {
      data: asset_data,
      alternate_names: Vec::new(),
      dependencies: Vec::new(),
      last_modified,
      cachable: true,
    };

    asset.resolve_dependencies(path);

    asset
  }

  pub fn load_from_file(path: &std::path::Path) -> Asset {
    use std::fs::{metadata, read};

    let data = read(&path).unwrap_or_default();
    let asset_data = resolve_asset_data(path, &data);

    let mut last_modified = 0;

    if let Ok(file_meta) = metadata(&path) {
      if let Ok(time) = file_meta.modified() {
        last_modified = time
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap_or_default()
          .as_secs();
      }
    }

    let mut asset = Asset {
      data: asset_data,
      alternate_names: Vec::new(),
      dependencies: Vec::new(),
      last_modified,
      cachable: true,
    };

    asset.resolve_dependencies(path);

    asset
  }

  pub fn len(&self) -> usize {
    match &self.data {
      AssetData::Text(data) => data.len(),
      AssetData::Texture(data) => data.len(),
      AssetData::Audio(data) => data.len(),
      AssetData::Data(data) => data.len(),
    }
  }

  /// Resolves dependencies and alternate name. `load_from_*` functions automatically call this
  pub fn resolve_dependencies(&mut self, path: &std::path::Path) {
    let extension = path
      .extension()
      .unwrap_or_default()
      .to_str()
      .unwrap_or_default();

    match extension {
      "tsx" => self.resolve_tsx_dependencies(),
      "zip" => self.resolve_zip_dependencies(path),
      _ => {}
    }
  }

  fn resolve_tsx_dependencies(&mut self) {
    let data = if let AssetData::Text(data) = &self.data {
      data
    } else {
      return;
    };

    if let Ok(tileset_element) = data.parse::<minidom::Element>() {
      for child in tileset_element.children() {
        match child.name() {
          "image" => {
            let source = child.attr("source").unwrap_or_default();

            if source.starts_with("/server/") {
              self
                .dependencies
                .push(AssetDependency::AssetPath(source.to_string()))
            }
          }
          "tile" => {
            self.resolve_tile_dependencies(child);
          }
          _ => {}
        }
      }
    }
  }

  fn resolve_tile_dependencies(&mut self, tile_element: &minidom::Element) {
    let tile_type = if let Some(tile_type) = tile_element.attr("type") {
      tile_type
    } else {
      return;
    };

    let properties_element = if let Some(properties_element) =
      tile_element.get_child("properties", minidom::NSChoice::Any)
    {
      properties_element
    } else {
      return;
    };

    for property_element in properties_element.children() {
      #[allow(clippy::single_match)]
      match (tile_type, property_element.attr("name")) {
        ("Conveyor", Some("Sound Effect")) => {
          let value = property_element.attr("value").unwrap_or_default();

          if value.starts_with("/server/") {
            self
              .dependencies
              .push(AssetDependency::AssetPath(value.to_string()));
          }
        }
        _ => {}
      }
    }
  }

  fn resolve_zip_dependencies(&mut self, path: &std::path::Path) {
    let data = if let AssetData::Data(data) = &self.data {
      data
    } else {
      return;
    };

    use std::io::Cursor;
    use zip::ZipArchive;

    let data_cursor = Cursor::new(data);

    let mut archive = if let Ok(archive) = ZipArchive::new(data_cursor) {
      archive
    } else {
      return;
    };

    if let Ok(mut mob_file) = archive.by_name("mob.lua") {
      use std::io::Read;

      let mut entry_script = String::new();

      if mob_file.read_to_string(&mut entry_script).is_ok() {
        Asset::resolve_mob_dependencies(
          &mut self.alternate_names,
          &mut self.dependencies,
          path,
          entry_script,
        );
      }
    };
  }

  fn resolve_mob_dependencies(
    alternate_names: &mut Vec<AssetDependency>,
    dependencies: &mut Vec<AssetDependency>,
    path: &std::path::Path,
    entry_script: String,
  ) {
    let lua = rlua::Lua::new();

    let result = lua.context(|lua_ctx| -> rlua::Result<()> {
      lua_ctx.scope(|scope| -> rlua::Result<()> {
        let engine_table = lua_ctx.create_table()?;

        engine_table.set(
          "DefineCharacter",
          scope.create_function_mut(|_, name: String| {
            alternate_names.push(AssetDependency::ScriptedCharacter(name));
            Ok(())
          })?,
        )?;

        engine_table.set(
          "RequiresCharacter",
          scope.create_function_mut(|_, name: String| {
            dependencies.push(AssetDependency::ScriptedCharacter(name));
            Ok(())
          })?,
        )?;

        let globals = lua_ctx.globals();
        globals.set("_modpath", "")?;
        globals.set("Engine", engine_table)?;

        lua_ctx.load(&entry_script).exec()?;

        let load_scripts_func: rlua::Function = globals.get("load_scripts")?;
        load_scripts_func.call(())?;

        Ok(())
      })
    });

    if let Err(e) = result {
      println!(
        "Failed to load \"mob.lua\" in \"{}\":\n{}",
        path.display(),
        e
      );
    }
  }
}

pub fn get_player_texture_path(player_id: &str) -> String {
  String::from("/server/players/") + player_id + ".texture"
}

pub fn get_player_animation_path(player_id: &str) -> String {
  String::from("/server/players/") + player_id + ".animation"
}

pub fn get_player_mugshot_texture_path(player_id: &str) -> String {
  String::from("/server/players/") + player_id + "_mug.texture"
}

pub fn get_player_mugshot_animation_path(player_id: &str) -> String {
  String::from("/server/players/") + player_id + "_mug.animation"
}

pub fn get_map_path(map_id: &str) -> String {
  String::from("/server/maps/") + map_id + ".tmx"
}

fn resolve_asset_data(path: &std::path::Path, data: &[u8]) -> AssetData {
  let extension = path
    .extension()
    .unwrap_or_default()
    .to_str()
    .unwrap_or_default();

  match extension {
    "png" | "bmp" => AssetData::Texture(data.to_vec()),
    "ogg" => AssetData::Audio(data.to_vec()),
    "zip" => AssetData::Data(data.to_vec()),
    "tsx" => {
      let original_data = String::from_utf8_lossy(data);
      let translated_data = translate_tsx(path, &original_data);

      if translated_data == None {
        println!("Invalid .tsx file: {:?}", path);
      }

      AssetData::Text(translated_data.unwrap_or_else(|| original_data.to_string()))
    }
    _ => AssetData::Text(String::from_utf8_lossy(data).to_string()),
  }
}

fn translate_tsx(path: &std::path::Path, data: &str) -> Option<String> {
  use crate::helpers::normalize_path;

  let root_path = std::path::Path::new("/server");
  let path_base = path.parent()?;
  let mut tileset_element = data.parse::<minidom::Element>().ok()?;

  for child in tileset_element.children_mut() {
    if child.name() != "image" {
      continue;
    }

    let source = path_base.join(child.attr("source")?);
    let mut normalized_source = normalize_path(&source);

    if normalized_source.starts_with("assets") {
      // path did not escape server folders
      normalized_source = root_path.join(normalized_source);
    }

    // adjust windows paths
    let corrected_source = normalized_source.to_string_lossy().replace('\\', "/");

    child.set_attr("source", corrected_source);
  }

  let mut output: Vec<u8> = Vec::new();

  tileset_element.write_to(&mut output).ok()?;

  Some(String::from_utf8_lossy(&output[..]).into_owned())
}
