use super::job_promise::{JobPromise, PromiseValue};
use super::Job;

pub fn write_file(path: String, content: &[u8]) -> (Job, JobPromise) {
  let promise = JobPromise::new();
  let mut thread_promise = promise.clone();

  // own content for thread
  let content = content.to_vec();

  let job = Box::new(move || {
    use std::fs::write;

    let success = write(path, content).is_ok();

    thread_promise.set_value(PromiseValue::Success(success));
  });

  (job, promise)
}
