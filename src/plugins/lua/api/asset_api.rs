use super::LuaApi;
use crate::net::{Asset, AssetData};

pub fn inject_dynamic(lua_api: &mut LuaApi) {
  lua_api.add_dynamic_function("Net", "update_asset", |api_ctx, lua_ctx, params| {
    let (path, data): (String, rlua::String) = lua_ctx.unpack_multi(params)?;
    use crate::net::asset::{resolve_asset_data, resolve_dependencies};

    let mut net = api_ctx.net_ref.borrow_mut();

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

    lua_ctx.pack_multi(())
  });

  lua_api.add_dynamic_function("Net", "has_asset", |api_ctx, lua_ctx, params| {
    let path: rlua::String = lua_ctx.unpack_multi(params)?;
    let path_str = path.to_str()?;
    let net = api_ctx.net_ref.borrow();

    let has_asset = net.get_asset(path_str).is_some();

    lua_ctx.pack_multi(has_asset)
  });

  lua_api.add_dynamic_function("Net", "get_asset_type", |api_ctx, lua_ctx, params| {
    let path: rlua::String = lua_ctx.unpack_multi(params)?;
    let path_str = path.to_str()?;
    let net = api_ctx.net_ref.borrow();

    let asset_type = if let Some(asset) = net.get_asset(path_str) {
      match asset.data {
        AssetData::Text(_) => Some("text"),
        AssetData::Texture(_) => Some("texture"),
        AssetData::Audio(_) => Some("audio"),
      }
    } else {
      None
    };

    lua_ctx.pack_multi(asset_type)
  });

  lua_api.add_dynamic_function("Net", "get_asset_size", |api_ctx, lua_ctx, params| {
    let path: rlua::String = lua_ctx.unpack_multi(params)?;
    let path_str = path.to_str()?;
    let net = api_ctx.net_ref.borrow();

    if let Some(asset) = net.get_asset(path_str) {
      lua_ctx.pack_multi(asset.len())
    } else {
      lua_ctx.pack_multi(0)
    }
  });
}
