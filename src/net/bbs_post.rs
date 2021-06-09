#[derive(Debug)]
pub struct BbsPost {
  pub id: String,
  pub read: bool,
  pub title: String,
  pub author: String,
}

pub fn calc_size(post: &BbsPost) -> usize {
  let id_size = post.id.len() + 1;
  let read_size = 1;
  let title_size = post.title.len() + 1;
  let author_size = post.author.len() + 1;

  id_size + read_size + title_size + author_size
}
