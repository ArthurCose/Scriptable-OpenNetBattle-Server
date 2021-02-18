#[derive(Clone, Debug)]
pub struct Asset {
  pub data: AssetData,
  pub dependencies: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum AssetData {
  Text(String),
  Texture(Vec<u8>),
  Audio(Vec<u8>),
  SFMLImage(Vec<u8>),
}

pub fn get_player_texture_path(player_id: &String) -> String {
  String::from("/server/navis/") + player_id + ".texture"
}

pub fn get_player_animation_path(player_id: &String) -> String {
  String::from("/server/navis/") + player_id + ".animation"
}

pub fn get_map_path(map_id: &String) -> String {
  String::from("/server/maps/") + map_id + ".tmx"
}

pub fn get_flattened_dependency_chain<'a>(
  assets: &'a std::collections::HashMap<String, Asset>,
  asset_path: &'a String,
) -> Vec<&'a String> {
  let mut chain = Vec::new();
  build_flattened_dependency_chain_with_recursion(assets, asset_path, &mut chain);
  chain
}

fn build_flattened_dependency_chain_with_recursion<'a>(
  assets: &'a std::collections::HashMap<String, Asset>,
  asset_path: &'a String,
  chain: &mut Vec<&'a String>,
) {
  if let Some(asset) = assets.get(asset_path) {
    for dependency_path in &asset.dependencies {
      if chain.contains(&dependency_path) {
        continue;
      }

      build_flattened_dependency_chain_with_recursion(assets, dependency_path, chain);
    }

    chain.push(asset_path);
  }
}

pub(super) fn translate_tsx(path: &std::path::PathBuf, data: &String) -> Option<String> {
  use crate::helpers::normalize_path;

  let root_path = std::path::Path::new("/server");
  let path_base = path.parent()?;
  let mut tileset_element = data.parse::<minidom::Element>().ok()?;

  for child in tileset_element.children_mut() {
    if child.name() == "image" {
      let source = path_base.join(child.attr("source")?);
      let normalized_source = normalize_path(&source);

      if normalized_source.starts_with("assets") {
        // path did not escape server folders
        child.set_attr(
          "source",
          root_path
            .join(normalized_source)
            .to_string_lossy()
            .into_owned(),
        );
      }
    }
  }

  let mut output: Vec<u8> = Vec::new();

  tileset_element.write_to(&mut output).ok()?;

  Some(String::from_utf8_lossy(&output[..]).into_owned())
}

pub(super) fn resolve_tsx_dependencies(data: &String) -> Vec<String> {
  let mut dependencies = Vec::new();

  if let Ok(tileset_element) = data.parse::<minidom::Element>() {
    for child in tileset_element.children() {
      if child.name() == "image" {
        if let Some(source) = child.attr("source") {
          dependencies.push(source.to_string());
        }
      }
    }
  }

  dependencies
}
