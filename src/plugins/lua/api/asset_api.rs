use crate::net::{AssetData, Net};
use std::cell::RefCell;

pub fn add_asset_api<'a, 'b>(
  api_table: &rlua::Table<'a>,
  scope: &rlua::Scope<'a, 'b>,
  net_ref: &'b RefCell<&mut Net>,
) -> rlua::Result<()> {
  api_table.set(
    "load_asset",
    scope.create_function(move |_, path: String| {
      let mut net = net_ref.borrow_mut();

      net.load_asset(path);

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
