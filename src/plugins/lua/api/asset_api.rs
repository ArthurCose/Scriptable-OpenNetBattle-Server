use crate::net::{Asset, AssetData, Net};
use std::cell::RefCell;

pub fn add_asset_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "update_asset",
    scope.create_function(move |_, (path, data): (String, rlua::String)| {
      use crate::net::asset::{resolve_asset_data, resolve_dependencies};

      let mut net = net_ref.borrow_mut();

      let path_buf = std::path::PathBuf::from(path.to_string());
      let asset_data = resolve_asset_data(&path_buf, data.as_bytes());
      let dependencies = resolve_dependencies(&path_buf, &asset_data);
      let last_modified = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Current time is before epoch?")
        .as_secs();

      let asset = Asset {
        data: asset_data,
        dependencies,
        last_modified,
        cachable: true,
      };

      net.set_asset(path, asset);

      Ok(())
    })?,
  )?;

  api_table.set(
    "has_asset",
    scope.create_function(move |_, path: String| {
      let net = net_ref.borrow();

      let has_asset = net.get_asset(&path).is_some();

      Ok(has_asset)
    })?,
  )?;

  api_table.set(
    "get_asset_type",
    scope.create_function(move |_, path: String| {
      let net = net_ref.borrow();

      if let Some(asset) = net.get_asset(&path) {
        match asset.data {
          AssetData::Text(_) => Ok(Some("text")),
          AssetData::Texture(_) => Ok(Some("texture")),
          AssetData::Audio(_) => Ok(Some("audio")),
        }
      } else {
        Ok(None)
      }
    })?,
  )?;

  api_table.set(
    "get_asset_size",
    scope.create_function(move |_, path: String| {
      let net = net_ref.borrow();

      if let Some(asset) = net.get_asset(&path) {
        Ok(asset.len())
      } else {
        Ok(0)
      }
    })?,
  )?;

  Ok(())
}
