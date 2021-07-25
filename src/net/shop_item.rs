#[derive(Debug)]
pub struct ShopItem {
  pub name: String,
  pub description: String,
  pub price: u32,
}

pub fn calc_size(item: &ShopItem) -> usize {
  let name_size = 1 + item.name.len(); // u8 size + characters
  let description_size = 1 + item.description.len(); // u8 size + characters
  let price_size = 2; // u16

  name_size + description_size + price_size
}
