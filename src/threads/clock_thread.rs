use crate::threads::ThreadMessage;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

pub const TICK_RATE: f64 = 20.0;

pub fn create_clock_thread(tx: mpsc::Sender<ThreadMessage>) {
  let target = std::time::Duration::from_secs_f64(1.0 / TICK_RATE);
  let behind_counter = Arc::new(AtomicU8::new(0));

  std::thread::spawn(move || loop {
    std::thread::sleep(target);

    let behind_count = behind_counter.fetch_add(1, Ordering::Relaxed);

    if behind_count > 1 {
      behind_counter.fetch_sub(1, Ordering::Relaxed);
      println!("Server running behind, skipping tick");
      continue;
    }

    let counter_rc_copy = behind_counter.clone();
    let start_callback = Box::new(move || {
      counter_rc_copy.fetch_sub(1, Ordering::Relaxed);
    });

    tx.send(ThreadMessage::Tick(start_callback)).unwrap();
  });
}
