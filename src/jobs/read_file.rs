use super::job_promise::{JobPromise, PromiseValue};
use super::Job;

pub fn read_file(path: String) -> (Job, JobPromise) {
  let promise = JobPromise::new();
  let mut thread_promise = promise.clone();

  let job = Box::new(move || {
    use std::fs::read;

    let contents = read(path).ok().unwrap_or_default();

    thread_promise.set_value(PromiseValue::Bytes(contents));
  });

  (job, promise)
}
