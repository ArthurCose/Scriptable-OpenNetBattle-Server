use super::job_promise::{JobPromise, PromiseValue};
use super::Job;

pub struct HttpResponse {
  pub status: u16,
  pub body: String,
  pub headers: Vec<(String, String)>,
}

pub fn web_request(
  url: String,
  method: String,
  headers: Vec<(String, String)>,
  body: Option<String>,
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

    let result = if let Some(body) = body {
      request.send_string(&body)
    } else {
      request.call()
    };

    if let Ok(response) = result {
      let status = response.status();
      let mut headers = Vec::new();

      for header_name in &response.headers_names() {
        let value = response.header(&header_name).unwrap().to_string();

        headers.push((header_name.clone(), value));
      }

      let body = response.into_string().unwrap_or_default();

      thread_promise.set_value(PromiseValue::HttpResponse(HttpResponse {
        status,
        body,
        headers,
      }));
    } else {
      thread_promise.set_value(PromiseValue::None);
    }
  });

  (job, promise)
}
