use super::job_promise::{JobPromise, PromiseValue};
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter};

pub fn web_download(
  destination: String,
  url: String,
  method: String,
  headers: Vec<(String, String)>,
  body: Option<Vec<u8>>,
) -> JobPromise {
  let promise = JobPromise::new();
  let mut thread_promise = promise.clone();

  async_std::task::spawn(async move {
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

    // handling response

    let response = match result {
      Ok(response) => response,
      Err(err) => {
        println!("{}", err);

        thread_promise.set_value(PromiseValue::Success(false));
        return;
      }
    };

    let status = response.status();

    if status != 200 {
      thread_promise.set_value(PromiseValue::Success(false));
      return;
    }

    let mut headers = Vec::new();

    for header_name in &response.headers_names() {
      let value = response.header(&header_name).unwrap().to_string();

      headers.push((header_name.clone(), value));
    }

    // writing to file

    let file = if let Ok(file) = File::create(destination) {
      file
    } else {
      thread_promise.set_value(PromiseValue::Success(false));
      return;
    };

    let reader = response.into_reader();
    let mut buf_reader = BufReader::new(reader);
    let mut buf_writer = BufWriter::new(file);

    let mut length = 1;

    while length > 0 {
      let buffer = buf_reader.fill_buf().unwrap();

      if buf_writer.write(buffer).is_err() {
        thread_promise.set_value(PromiseValue::Success(false));
        return;
      }

      length = buffer.len();
      buf_reader.consume(length);
    }

    thread_promise.set_value(PromiseValue::Success(true));
  });

  promise
}
