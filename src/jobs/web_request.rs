use super::job_promise::{JobPromise, PromiseValue};
use log::*;

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
) -> JobPromise {
  let promise = JobPromise::new();
  let mut thread_promise = promise.clone();

  async_std::task::spawn(async move {
    let mut response = match web_request_internal(url, method, headers, body).await {
      Ok(response) => response,
      Err(err) => {
        warn!("{}", err);
        thread_promise.set_value(PromiseValue::None);
        return;
      }
    };

    let status: u16 = response.status().into();
    let mut headers = Vec::new();

    for (name, value) in response.iter() {
      headers.push((String::from(name.as_str()), String::from(value.as_str())));
    }

    let body = match response.body_bytes().await {
      Ok(body) => body,
      Err(err) => {
        warn!("{}", err);

        thread_promise.set_value(PromiseValue::None);
        return;
      }
    };

    thread_promise.set_value(PromiseValue::HttpResponse(HttpResponse {
      status,
      body,
      headers,
    }));
  });

  promise
}

pub(super) async fn web_request_internal(
  url: String,
  method: String,
  headers: Vec<(String, String)>,
  body: Option<Vec<u8>>,
) -> Result<surf::Response, Box<dyn std::error::Error>> {
  use std::str::FromStr;
  use surf::http::url::Url;
  use surf::http::Method;

  let url = Url::parse(&url)?;

  let method = Method::from_str(&method).unwrap_or(Method::Get);

  let mut request = surf::RequestBuilder::new(method, url);

  for (key, value) in headers {
    request = request.header(key.as_str(), value.as_str());
  }

  if let Some(data) = body {
    request = request.body(data);
  }

  // handling response
  let response = request.await?;

  Ok(response)
}
