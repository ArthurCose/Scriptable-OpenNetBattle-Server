use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub type Job = Box<dyn FnOnce() + std::marker::Send>;

pub struct JobGiver {
  sender: Sender<Job>,
}

impl JobGiver {
  pub fn give_job(&mut self, job: Job) {
    self.sender.send(job).unwrap();
  }
}

pub fn create_worker_threads(count: u16) -> JobGiver {
  let (sender, receiver): (Sender<Job>, Receiver<Job>) = channel();

  let receiver_mutex = Arc::new(Mutex::new(receiver));

  for _ in 0..count {
    let receiver_mutex = receiver_mutex.clone();

    std::thread::spawn(move || loop {
      let job = {
        let lock = receiver_mutex.lock().unwrap();
        let receiver = &*lock;

        receiver.recv().unwrap()
      };

      job();
    });
  }

  JobGiver { sender }
}
