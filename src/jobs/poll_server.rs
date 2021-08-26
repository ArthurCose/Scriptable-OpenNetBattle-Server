use super::job_promise::{JobPromise, PromiseValue};
use crate::packets::{VERSION_ID, VERSION_ITERATION};
use async_std::net::UdpSocket;

pub fn poll_server(address: String, port: u16) -> JobPromise {
  let promise = JobPromise::new();
  let mut thread_promise = promise.clone();

  async_std::task::spawn(async move {
    use super::helpers::*;
    use futures::FutureExt;
    use std::time::Duration;

    let socket_addr = if let Some(socket_addr) = resolve_socket_addr(address.as_str(), port).await {
      socket_addr
    } else {
      thread_promise.set_value(PromiseValue::None);
      return;
    };

    let socket = if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
      socket
    } else {
      thread_promise.set_value(PromiseValue::None);
      return;
    };

    // only send + recieve to this address
    if socket.connect(socket_addr).await.is_err() {
      // invalid address
      thread_promise.set_value(PromiseValue::None);
      return;
    }

    let max_wait = Duration::from_millis(500);
    let mut attempts: u8 = 0;

    while attempts < 10 {
      // send &[unreliable, enum_part, enum_part]
      let _ = socket.send(&[0, 0, 0]).await;

      let response_fut = get_response(&socket).fuse();
      let timeout_fut = async_std::task::sleep(max_wait).fuse();

      futures::pin_mut!(response_fut, timeout_fut);

      futures::select! {
        result = response_fut => {
          if let Some(max_message_size) = result {
            thread_promise.set_value(PromiseValue::ServerInfo { max_message_size });
            return;
          }
        },
        () = timeout_fut => {}
      };

      attempts += 1;
    }

    thread_promise.set_value(PromiseValue::None);
  });

  promise
}

async fn get_response(socket: &UdpSocket) -> Option<u16> {
  use crate::packets::bytes::*;

  // max size defined by NetPlayConfig::MAX_BUFFER_LEN
  let mut buf = [0; 10240];

  if let Ok(size) = socket.recv(&mut buf).await {
    let slice = &mut &buf[..size];

    if !matches!(read_byte(slice), Some(0)) {
      // invalid response: expecting "unreliable" byte
      return None;
    }

    if !matches!(read_u16(slice), Some(0)) {
      // invalid response: expecting "VersionInfo" byte
      return None;
    }

    if let Some(version_id_optional) = read_string_u16(slice) {
      if version_id_optional != VERSION_ID {
        // invalid response: mismatching VERSION_ID
        return None;
      }
      // good path
    } else {
      // invalid response: expecting VERSION_ID
      return None;
    }

    if !matches!(read_u64(slice), Some(VERSION_ITERATION)) {
      // invalid response: mismatching VERSION_ITERATION
      return None;
    }

    let max_payload_size = read_u16(slice)?;

    // header size = unreliable byte + packet type u16
    let header_size = 1 + 2;
    let max_message_size = max_payload_size - header_size;

    return Some(max_message_size);
  }

  None
}
