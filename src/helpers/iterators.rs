pub struct LossyChunkPacker<I, F>
where
  I: Iterator,
  F: Fn(&I::Item) -> usize,
{
  iterator: I,
  current_chunk: Vec<I::Item>,
  measure_item: F,
  size_per_chunk: usize,
  remaining_size: usize,
}

impl<I, F> LossyChunkPacker<I, F>
where
  I: Iterator,
  F: Fn(&I::Item) -> usize,
{
  fn flush(&mut self) -> Vec<I::Item> {
    let mut out = Vec::new();

    std::mem::swap(&mut self.current_chunk, &mut out);

    out
  }
}

impl<I, F> Iterator for LossyChunkPacker<I, F>
where
  I: Iterator,
  F: Fn(&I::Item) -> usize,
{
  type Item = Vec<I::Item>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      let item = match self.iterator.next() {
        Some(item) => item,
        None => {
          if self.current_chunk.is_empty() {
            return None;
          } else {
            return Some(self.flush());
          }
        }
      };

      let measure_item = &self.measure_item;
      let item_size = measure_item(&item);

      if item_size >= self.size_per_chunk {
        // too big for any chunk, ignore
        continue;
      }

      if self.remaining_size < item_size {
        // reset remaining size
        self.remaining_size = self.size_per_chunk;

        return Some(self.flush());
      }

      self.current_chunk.push(item);
      self.remaining_size -= item_size;
    }
  }
}

pub trait IteratorHelper: Iterator {
  fn pack_chunks_lossy<F>(self, size_per_chunk: usize, measure_item: F) -> LossyChunkPacker<Self, F>
  where
    F: Fn(&Self::Item) -> usize,
    Self: Sized,
  {
    LossyChunkPacker {
      iterator: self,
      current_chunk: Vec::new(),
      measure_item,
      size_per_chunk,
      remaining_size: size_per_chunk,
    }
  }
}

impl<I> IteratorHelper for I where I: Iterator {}
