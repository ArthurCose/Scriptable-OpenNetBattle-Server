mod job_promise;
mod job_promise_manager;
pub mod read_file;
pub mod web_download;
pub mod web_request;

pub use job_promise::JobPromise;
pub use job_promise::PromiseValue;
pub use job_promise_manager::JobPromiseManager;

pub type Job = Box<dyn FnOnce() + std::marker::Send>;
