pub fn unwrap_and_parse_or_default<A>(option: Option<&str>) -> A
where
  A: Default + std::str::FromStr,
{
  option.unwrap_or_default().parse().unwrap_or_default()
}

pub fn normalize_path(path: &std::path::PathBuf) -> std::path::PathBuf {
  let mut normalized_path: std::path::PathBuf = std::path::PathBuf::new();

  for component in path.components() {
    let component_as_os_str = component.as_os_str();

    if component_as_os_str == "." {
      continue;
    }

    if component_as_os_str == ".." {
      if normalized_path.file_name() == None {
        // file_name() returns none for ".." as last component and no component
        // this means we're building out the start like ../../../etc
        normalized_path.push("..");
      } else {
        normalized_path.pop();
      }
    } else {
      normalized_path.push(component);
    }
  }

  normalized_path
}
