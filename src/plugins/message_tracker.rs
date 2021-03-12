use std::collections::HashMap;
use std::collections::VecDeque;

// tracks what script owns what message
pub struct MessageTracker<T> {
  message_map: HashMap<String, VecDeque<T>>,
}

impl<T> MessageTracker<T> {
  pub fn new() -> MessageTracker<T> {
    MessageTracker {
      message_map: HashMap::new(),
    }
  }

  pub fn track_message(&mut self, player_id: &str, owner: T) {
    let optional_messages = self.message_map.get_mut(player_id);

    let messages;
    if let Some(unwrapped_messages) = optional_messages {
      messages = unwrapped_messages;
    } else {
      self
        .message_map
        .insert(player_id.to_string(), VecDeque::new());
      messages = self.message_map.get_mut(player_id).unwrap();
    };

    messages.push_back(owner);
  }

  pub fn pop_message(&mut self, player_id: &str) -> Option<T> {
    if let Some(messages) = self.message_map.get_mut(player_id) {
      messages.pop_front()
    } else {
      None
    }
  }

  // for disconnects
  pub fn remove_tracking(&mut self, player_id: &str) {
    self.message_map.remove(player_id);
  }
}
