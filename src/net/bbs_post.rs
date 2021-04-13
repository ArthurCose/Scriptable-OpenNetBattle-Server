#[derive(Debug)]
pub struct BBSPost {
  pub id: String,
  pub read: bool,
  pub title: String,
  pub author: String,
}

fn calc_size(post: &BBSPost) -> usize {
  let id_size = post.id.len() + 1;
  let read_size = 1;
  let title_size = post.title.len() + 1;
  let author_size = post.author.len() + 1;

  id_size + read_size + title_size + author_size
}

pub fn count_fit_posts(available_room: usize, start_index: usize, posts: &[BBSPost]) -> usize {
  let mut available_room = available_room as isize;

  for (i, post) in posts[start_index..].iter().enumerate() {
    let post_size = calc_size(post) as isize;

    if available_room - post_size < 0 {
      return i;
    }

    available_room -= post_size;
  }

  posts.len() - start_index
}
