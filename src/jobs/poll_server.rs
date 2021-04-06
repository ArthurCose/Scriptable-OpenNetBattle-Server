use super::job_promise::{JobPromise, PromiseValue};
use super::Job;

pub fn poll_server(address: std::net::SocketAddr) -> (Job, JobPromise) {
  let promise = JobPromise::new();
  let mut thread_promise = promise.clone();

  let job = Box::new(move || {
    use crate::packets::bytes::*;
    use crate::packets::{VERSION_ID, VERSION_ITERATION};
    use std::net::UdpSocket;

    let socket = if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
      socket
    } else {
      thread_promise.set_value(PromiseValue::None);
      return;
    };

    // only send + recieve to this address
    if socket.connect(address).is_err() {
      // invalid address
      thread_promise.set_value(PromiseValue::None);
      return;
    }

    let mut attempts = 0;

    // max size defined by NetPlayConfig::MAX_BUFFER_LEN
    let mut buf = [0; 10240];

    while attempts < 5 {
      // send &[unreliable, ping_part, ping_part]
      let _ = socket.send(&[0, 0, 0]);

      if let Ok(size) = socket.recv(&mut buf) {
        let slice = &mut &buf[..size];

        if !matches!(read_byte(slice), Some(0)) {
          // invalid response: expecting "unreliable" byte
          break;
        }

        if !matches!(read_u16(slice), Some(0)) {
          // invalid response: expecting "Pong" byte
          break;
        }

        if let Some(version_id_optional) = read_string(slice) {
          if version_id_optional != VERSION_ID {
            // invalid response: mismatching VERSION_ID
            break;
          }
          // good path
        } else {
          // invalid response: expecting VERSION_ID
          break;
        }

        if !matches!(read_u64(slice), Some(VERSION_ITERATION)) {
          // invalid response: mismatching VERSION_ITERATION
          break;
        }

        let max_payload_size = if let Some(max_payload_size) = read_u16(slice) {
          max_payload_size
        } else {
          // invalid response: missing max_payload_size
          break;
        };

        // header size = unreliable byte + packet type u16
        let header_size = 1 + 2;
        let max_message_size = max_payload_size - header_size;

        thread_promise.set_value(PromiseValue::ServerInfo { max_message_size });

        return;
      }

      attempts += 1;
    }

    thread_promise.set_value(PromiseValue::None);
  });

  (job, promise)
}
