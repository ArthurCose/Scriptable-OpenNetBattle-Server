#[derive(Clone, Debug)]
pub struct Asset {
  pub data: AssetData,
  pub dependencies: Vec<String>,
  pub last_modified: u64,
  pub cachable: bool, // allows the client to know if they can cache this asset or if it's dynamic
}

#[derive(Clone, Debug)]
pub enum AssetData {
  Text(String),
  Texture(Vec<u8>),
  Audio(Vec<u8>),
}

impl Asset {
  pub fn len(&self) -> usize {
    match &self.data {
      AssetData::Text(data) => data.len(),
      AssetData::Texture(data) => data.len(),
      AssetData::Audio(data) => data.len(),
    }
  }
}

pub fn get_player_texture_path(player_id: &str) -> String {
  String::from("/server/navis/") + player_id + ".texture"
}

pub fn get_player_animation_path(player_id: &str) -> String {
  String::from("/server/navis/") + player_id + ".animation"
}

pub fn get_map_path(map_id: &str) -> String {
  String::from("/server/maps/") + map_id + ".tmx"
}

pub(super) fn get_flattened_dependency_chain<'a>(
  assets: &'a std::collections::HashMap<String, Asset>,
  asset_path: &'a str,
) -> Vec<&'a str> {
  let mut chain = Vec::new();
  build_flattened_dependency_chain_with_recursion(assets, asset_path, &mut chain);
  chain
}

fn build_flattened_dependency_chain_with_recursion<'a>(
  assets: &'a std::collections::HashMap<String, Asset>,
  asset_path: &'a str,
  chain: &mut Vec<&'a str>,
) {
  if let Some(asset) = assets.get(asset_path) {
    for dependency_path in &asset.dependencies {
      if chain.contains(&&dependency_path[..]) {
        continue;
      }

      build_flattened_dependency_chain_with_recursion(assets, dependency_path, chain);
    }

    chain.push(asset_path);
  }
}

pub(super) fn load_asset(path: std::path::PathBuf) -> Asset {
  use std::fs::{metadata, read, read_to_string};

  let path_string = path.to_str().unwrap_or_default();
  let extension_index = path_string.rfind('.').unwrap_or_else(|| path_string.len());
  let extension = path_string.to_lowercase().split_off(extension_index);

  let asset_data = if extension == ".ogg" {
    AssetData::Audio(read(&path).unwrap_or_default())
  } else if extension == ".png" || extension == ".bmp" {
    AssetData::Texture(read(&path).unwrap_or_default())
  } else if extension == ".tsx" {
    let original_data = read_to_string(&path).unwrap_or_default();
    let translated_data = translate_tsx(&path, &original_data);

    if translated_data == None {
      println!("Invalid .tsx file: {:?}", path);
    }

    AssetData::Text(translated_data.unwrap_or(original_data))
  } else {
    AssetData::Text(read_to_string(&path).unwrap_or_default())
  };

  let mut dependencies = Vec::new();

  if extension == ".tsx" {
    // can't chain yet: https://github.com/rust-lang/rust/issues/53667
    if let AssetData::Text(data) = &asset_data {
      dependencies = resolve_tsx_dependencies(data);
    }
  }

  let mut last_modified = 0;

  if let Ok(file_meta) = metadata(path) {
    if let Ok(time) = file_meta.modified() {
      last_modified = time
        .duration_since(std::time::UNIX_EPOCH)
        .expect("File written before epoch?")
        .as_secs();
    }
  }

  Asset {
    data: asset_data,
    dependencies,
    last_modified,
    cachable: true,
  }
}

pub(super) fn translate_tsx(path: &std::path::PathBuf, data: &str) -> Option<String> {
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

pub(super) fn resolve_tsx_dependencies(data: &str) -> Vec<String> {
  if let Ok(tileset_element) = data.parse::<minidom::Element>() {
    return tileset_element
      .children()
      .filter(|child| child.name() == "image")
      .map(|child| child.attr("source").unwrap_or_default())
      .filter(|source| source.starts_with("/server"))
      .map(|source| source.to_string())
      .collect();
  }

  vec![]
}
