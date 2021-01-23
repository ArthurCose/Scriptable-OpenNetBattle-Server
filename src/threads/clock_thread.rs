use crate::threads::ThreadMessage;
use std::sync::mpsc;

pub fn create_clock_thread(tx: mpsc::Sender<ThreadMessage>) {
  let target = std::time::Duration::from_secs_f64(1.0 / 20.0);

  std::thread::spawn(move || loop {
    std::thread::sleep(target);
    tx.send(ThreadMessage::Tick).unwrap();
  });
}
