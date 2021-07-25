#[derive(Debug)]
pub struct BbsPost {
  pub id: String,
  pub read: bool,
  pub title: String,
  pub author: String,
}

pub fn calc_size(post: &BbsPost) -> usize {
  let id_size = 2 + post.id.len(); // u16 size + characters
  let read_size = 1; // bool
  let title_size = 2 + post.title.len(); // u16 size + characters
  let author_size = 2 + post.author.len(); // u16 size + characters

  id_size + read_size + title_size + author_size
}
