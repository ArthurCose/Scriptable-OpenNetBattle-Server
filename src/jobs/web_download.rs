use super::job_promise::{JobPromise, PromiseValue};
use super::web_request::web_request_internal;

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
    let response = match web_request_internal(url, method, headers, body).await {
      Ok(response) => response,
      Err(err) => {
        println!("{}", err);
        thread_promise.set_value(PromiseValue::Success(false));
        return;
      }
    };

    // writing to file
    if let Err(err) = save_response(destination, response).await {
      println!("{}", err);
      thread_promise.set_value(PromiseValue::Success(false));
      return;
    }

    thread_promise.set_value(PromiseValue::Success(true));
  });

  promise
}

// seems to break on mp4s
// test file:
// https://cdn.discordapp.com/attachments/820777515995234314/856179686211059712/2021-06-20_09-20-08.mp4
async fn save_response(
  destination: String,
  response: surf::Response,
) -> Result<(), Box<dyn std::error::Error>> {
  use async_std::fs::File;
  use async_std::io::BufWriter;
  use futures::{AsyncBufReadExt, AsyncWriteExt};

  let mut response = response;

  let file = File::create(destination).await?;

  let mut buf_reader = response.take_body().into_reader();
  let mut buf_writer = BufWriter::new(file);

  let mut length = 1;

  while length > 0 {
    let buffer = buf_reader.fill_buf().await?;
    length = buffer.len();

    buf_writer.write(buffer).await?;

    buf_reader.consume_unpin(length);
  }

  buf_writer.flush().await?;

  Ok(())
}
