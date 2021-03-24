use super::job_promise::{JobPromise, PromiseValue};
use super::Job;
use std::io::Read;

pub struct HttpResponse {
  pub status: u16,
  pub body: Vec<u8>,
  pub headers: Vec<(String, String)>,
}

pub fn web_request(
  url: String,
  method: String,
  headers: Vec<(String, String)>,
  body: Option<Vec<u8>>,
) -> (Job, JobPromise) {
  let promise = JobPromise::new();
  let mut thread_promise = promise.clone();

  let job = Box::new(move || {
    // todo: there's more methods than this
    // can i just set the method in headers?
    let mut request = match method.as_str() {
      "post" => ureq::post(&url),
      "put" => ureq::put(&url),
      "delete" => ureq::delete(&url),
      _ => ureq::get(&url),
    };

    for (key, value) in headers {
      request = request.set(key.as_str(), value.as_str());
    }

    let result = if let Some(data) = body {
      request.send_bytes(&data)
    } else {
      request.call()
    };

    let response = match result {
      Ok(response) => response,
      Err(err) => {
        println!("{}", err);

        thread_promise.set_value(PromiseValue::None);
        return;
      }
    };

    let status = response.status();
    let mut headers = Vec::new();

    for header_name in &response.headers_names() {
      let value = response.header(&header_name).unwrap().to_string();

      headers.push((header_name.clone(), value));
    }

    let mut body = Vec::new();

    if let Err(err) = response.into_reader().read_to_end(&mut body) {
      println!("{}", err);

      thread_promise.set_value(PromiseValue::None);
      return;
    }

    thread_promise.set_value(PromiseValue::HttpResponse(HttpResponse {
      status,
      body,
      headers,
    }));
  });

  (job, promise)
}
