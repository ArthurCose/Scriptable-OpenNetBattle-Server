use super::Net;
use crate::plugins::PluginInterface;

pub(super) struct PluginWrapper {
  plugin_interfaces: Vec<Box<dyn PluginInterface>>,
}

impl PluginWrapper {
  pub(super) fn new() -> PluginWrapper {
    PluginWrapper {
      plugin_interfaces: Vec::new(),
    }
  }

  pub(super) fn add_plugin_interface(&mut self, plugin_interface: Box<dyn PluginInterface>) {
    self.plugin_interfaces.push(plugin_interface);
  }

  fn wrap_call<C>(&mut self, i: usize, net: &mut Net, call: C)
  where
    C: FnMut(&mut Box<dyn PluginInterface>, &mut Net),
  {
    let mut call = call;

    net.set_active_script(i);
    call(&mut self.plugin_interfaces[i], net);
  }

  fn wrap_calls<C>(&mut self, net: &mut Net, call: C)
  where
    C: FnMut(&mut Box<dyn PluginInterface>, &mut Net),
  {
    let mut call = call;

    for (i, plugin_interface) in self.plugin_interfaces.iter_mut().enumerate() {
      net.set_active_script(i);
      call(plugin_interface, net);
    }
  }
}

impl PluginInterface for PluginWrapper {
  fn init(&mut self, net: &mut Net) {
    self.wrap_calls(net, |plugin_interface, net| plugin_interface.init(net));
  }

  fn tick(&mut self, net: &mut Net, delta_time: f32) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.tick(net, delta_time)
    });
  }

  fn handle_player_connect(&mut self, net: &mut Net, player_id: &str) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_player_connect(net, player_id)
    });
  }

  fn handle_player_join(&mut self, net: &mut Net, player_id: &str) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_player_join(net, player_id)
    });
  }

  fn handle_player_transfer(&mut self, net: &mut Net, player_id: &str) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_player_transfer(net, player_id)
    });
  }

  fn handle_player_disconnect(&mut self, net: &mut Net, player_id: &str) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_player_disconnect(net, player_id)
    });
  }

  fn handle_player_move(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_player_move(net, player_id, x, y, z)
    });
  }

  fn handle_player_avatar_change(
    &mut self,
    net: &mut Net,
    player_id: &str,
    texture_path: &str,
    animation_path: &str,
  ) -> bool {
    let mut prevent_default = false;

    self.wrap_calls(net, |plugin_interface, net| {
      prevent_default |=
        plugin_interface.handle_player_avatar_change(net, player_id, texture_path, animation_path)
    });

    prevent_default
  }

  fn handle_player_emote(&mut self, net: &mut Net, player_id: &str, emote_id: u8) -> bool {
    let mut prevent_default = false;

    self.wrap_calls(net, |plugin_interface, net| {
      prevent_default |= plugin_interface.handle_player_emote(net, player_id, emote_id)
    });

    prevent_default
  }

  fn handle_object_interaction(&mut self, net: &mut Net, player_id: &str, tile_object_id: u32) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_object_interaction(net, player_id, tile_object_id)
    });
  }

  fn handle_actor_interaction(&mut self, net: &mut Net, player_id: &str, actor_id: &str) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_actor_interaction(net, player_id, actor_id)
    });
  }

  fn handle_tile_interaction(&mut self, net: &mut Net, player_id: &str, x: f32, y: f32, z: f32) {
    self.wrap_calls(net, |plugin_interface, net| {
      plugin_interface.handle_tile_interaction(net, player_id, x, y, z)
    });
  }

  fn handle_dialog_response(&mut self, net: &mut Net, player_id: &str, response: u8) {
    let client = net
      .get_client_mut(player_id)
      .expect("An internal author should understand how to handle this better");

    // expect the above to be correct
    // don't expect the client to be correct
    // otherwise someone can read the source and force a crash :p
    if let Some(i) = client.pop_message() {
      self.wrap_call(i, net, |plugin_interface, net| {
        plugin_interface.handle_dialog_response(net, player_id, response)
      });
    }
  }
}
