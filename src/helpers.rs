pub fn unwrap_and_parse_or_default<A>(option: Option<&str>) -> A
where
  A: Default + std::str::FromStr,
{
  option.unwrap_or_default().parse().unwrap_or_default()
}
