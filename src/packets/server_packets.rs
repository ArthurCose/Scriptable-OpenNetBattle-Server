// Increment VERSION_ITERATION src/packets/mod.rs if packets are added or modified

use super::bytes::*;
use super::{VERSION_ID, VERSION_ITERATION};
use crate::net::actor_property_animation::{ActorProperty, Ease, KeyFrame};
use crate::net::{Asset, AssetData, BbsPost, Direction, PackageCategory, PackageInfo, ShopItem};

#[repr(u16)]
enum ServerPacketId {
  VersionInfo,
  Ack,
  Heartbeat,
  Authorize,
  Login,
  CompleteConnection,
  TransferWarp,
  TransferStart,
  TransferComplete,
  TransferServer,
  Kick,
  RemoveAsset,
  AssetStreamStart,
  AssetStream,
  Preload,
  CustomEmotesPath,
  MapUpdate,
  Health,
  Emotion,
  Money,
  AddItem,
  RemoveItem,
  PlaySound,
  ExcludeObject,
  IncludeObject,
  ExcludeActor,
  IncludeActor,
  MoveCamera,
  SlideCamera,
  ShakeCamera,
  FadeCamera,
  TrackWithCamera,
  UnlockCamera,
  LockInput,
  UnlockInput,
  Teleport,
  Message,
  Question,
  Quiz,
  Prompt,
  OpenBoard,
  PrependPosts,
  AppendPosts,
  RemovePost,
  PostSelectionAck,
  CloseBBS,
  ShopInventory,
  OpenShop,
  LoadPackage,
  ModWhitelist,
  InitiateEncounter,
  InitiatePvp,
  ActorConnected,
  ActorDisconnected,
  ActorSetName,
  ActorMove,
  ActorSetAvatar,
  ActorEmote,
  ActorAnimate,
  ActorPropertyKeyFrames,
  ActorMinimapColor,
  OfferPackage,
}

#[derive(Debug)]
pub enum ServerPacket<'a> {
  VersionInfo {
    max_payload_size: usize,
  },
  Ack {
    reliability: u8,
    id: u64,
  },
  Heartbeat,
  Authorize {
    address: &'a str,
    port: u16,
    data: &'a [u8],
  },
  Login {
    ticket: &'a str,
    warp_in: bool,
    spawn_x: f32,
    spawn_y: f32,
    spawn_z: f32,
    spawn_direction: Direction,
  },
  CompleteConnection,
  TransferWarp,
  TransferStart,
  TransferComplete {
    warp_in: bool,
    direction: Direction,
  },
  TransferServer {
    address: &'a str,
    port: u16,
    data: &'a str,
    warp_out: bool,
  },
  Kick {
    reason: &'a str,
  },
  RemoveAsset {
    path: &'a str,
  },
  AssetStreamStart {
    name: &'a str,
    asset: &'a Asset,
  },
  AssetStream {
    data: &'a [u8],
  },
  Preload {
    asset_path: &'a str,
  },
  CustomEmotesPath {
    asset_path: &'a str,
  },
  MapUpdate {
    map_path: &'a str,
  },
  Health {
    health: u32,
    max_health: u32,
  },
  Emotion {
    emotion: u8,
  },
  Money {
    money: u32,
  },
  AddItem {
    id: &'a str,
    name: &'a str,
    description: &'a str,
  },
  RemoveItem {
    id: &'a str,
  },
  PlaySound {
    path: &'a str,
  },
  ExcludeObject {
    id: u32,
  },
  IncludeObject {
    id: u32,
  },
  ExcludeActor {
    actor_id: &'a str,
  },
  IncludeActor {
    actor_id: &'a str,
  },
  MoveCamera {
    x: f32,
    y: f32,
    z: f32,
    hold_time: f32,
  },
  SlideCamera {
    x: f32,
    y: f32,
    z: f32,
    duration: f32,
  },
  ShakeCamera {
    strength: f32,
    duration: f32,
  },
  FadeCamera {
    duration: f32,
    color: (u8, u8, u8, u8),
  },
  TrackWithCamera {
    actor_id: Option<&'a str>,
  },
  UnlockCamera,
  LockInput,
  UnlockInput,
  Teleport {
    warp: bool,
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  },
  Message {
    message: &'a str,
    mug_texture_path: &'a str,
    mug_animation_path: &'a str,
  },
  Question {
    message: &'a str,
    mug_texture_path: &'a str,
    mug_animation_path: &'a str,
  },
  Quiz {
    option_a: &'a str,
    option_b: &'a str,
    option_c: &'a str,
    mug_texture_path: &'a str,
    mug_animation_path: &'a str,
  },
  Prompt {
    character_limit: u16,
    default_text: Option<&'a str>,
  },
  OpenBoard {
    current_depth: u8,
    name: &'a str,
    color: (u8, u8, u8),
    posts: &'a [BbsPost],
  },
  PrependPosts {
    current_depth: u8,
    reference: Option<&'a str>,
    posts: &'a [BbsPost],
  },
  AppendPosts {
    current_depth: u8,
    reference: Option<&'a str>,
    posts: &'a [BbsPost],
  },
  RemovePost {
    current_depth: u8,
    id: &'a str,
  },
  PostSelectionAck,
  CloseBBS,
  ShopInventory {
    items: &'a [ShopItem],
  },
  OpenShop {
    mug_texture_path: &'a str,
    mug_animation_path: &'a str,
  },
  OfferPackage {
    package_info: &'a PackageInfo,
    package_path: &'a str,
  },
  LoadPackage {
    package_path: &'a str,
    package_category: PackageCategory,
  },
  ModWhitelist {
    whitelist_path: &'a str,
  },
  InitiateEncounter {
    package_path: &'a str,
    data_path: &'a str,
  },
  InitiatePvp {
    address: &'a str,
  },
  ActorConnected {
    ticket: &'a str,
    name: &'a str,
    texture_path: &'a str,
    animation_path: &'a str,
    direction: Direction,
    x: f32,
    y: f32,
    z: f32,
    solid: bool,
    warp_in: bool,
    scale_x: f32,
    scale_y: f32,
    rotation: f32,
    minimap_color: (u8, u8, u8, u8),
    animation: Option<&'a str>,
  },
  ActorDisconnected {
    ticket: &'a str,
    warp_out: bool,
  },
  ActorSetName {
    ticket: &'a str,
    name: &'a str,
  },
  ActorMove {
    ticket: &'a str,
    x: f32,
    y: f32,
    z: f32,
    direction: Direction,
  },
  ActorSetAvatar {
    ticket: &'a str,
    texture_path: &'a str,
    animation_path: &'a str,
  },
  ActorEmote {
    ticket: &'a str,
    emote_id: u8,
    use_custom_emotes: bool,
  },
  ActorAnimate {
    ticket: &'a str,
    state: &'a str,
    loop_animation: bool,
  },
  ActorPropertyKeyFrames {
    ticket: &'a str,
    tail: bool,
    keyframes: Vec<KeyFrame>,
  },
  ActorMinimapColor {
    ticket: &'a str,
    color: (u8, u8, u8, u8),
  },
}

pub fn build_unreliable_packet(packet: ServerPacket) -> Vec<u8> {
  let mut buf = build_packet(packet);
  buf.insert(0, 0);
  buf
}

pub fn build_packet(packet: ServerPacket) -> Vec<u8> {
  let mut vec = Vec::new();
  let buf = &mut vec;

  match packet {
    ServerPacket::VersionInfo { max_payload_size } => {
      write_u16(buf, ServerPacketId::VersionInfo as u16);
      write_string_u16(buf, VERSION_ID);
      write_u64(buf, VERSION_ITERATION);
      write_u16(buf, max_payload_size as u16);
    }
    ServerPacket::Ack { reliability, id } => {
      write_u16(buf, ServerPacketId::Ack as u16);
      buf.push(reliability);
      write_u64(buf, id);
    }
    ServerPacket::Heartbeat => {
      write_u16(buf, ServerPacketId::Heartbeat as u16);
    }
    ServerPacket::Authorize {
      address,
      port,
      data,
    } => {
      write_u16(buf, ServerPacketId::Authorize as u16);
      write_string_u16(buf, address);
      write_u16(buf, port);
      buf.extend(data);
    }
    ServerPacket::Login {
      ticket,
      warp_in,
      spawn_x,
      spawn_y,
      spawn_z,
      spawn_direction,
    } => {
      write_u16(buf, ServerPacketId::Login as u16);
      write_string_u16(buf, ticket);
      write_bool(buf, warp_in);
      write_f32(buf, spawn_x);
      write_f32(buf, spawn_y);
      write_f32(buf, spawn_z);
      buf.push(translate_direction(spawn_direction));
    }
    ServerPacket::CompleteConnection => {
      write_u16(buf, ServerPacketId::CompleteConnection as u16);
    }
    ServerPacket::TransferWarp => {
      write_u16(buf, ServerPacketId::TransferWarp as u16);
    }
    ServerPacket::TransferStart => {
      write_u16(buf, ServerPacketId::TransferStart as u16);
    }
    ServerPacket::TransferComplete { warp_in, direction } => {
      write_u16(buf, ServerPacketId::TransferComplete as u16);
      write_bool(buf, warp_in);
      buf.push(translate_direction(direction));
    }
    ServerPacket::TransferServer {
      address,
      port,
      data,
      warp_out,
    } => {
      write_u16(buf, ServerPacketId::TransferServer as u16);
      write_string_u16(buf, address);
      write_u16(buf, port);
      write_string_u16(buf, data);
      write_bool(buf, warp_out);
    }
    ServerPacket::Kick { reason } => {
      write_u16(buf, ServerPacketId::Kick as u16);
      write_string_u16(buf, reason);
    }
    ServerPacket::RemoveAsset { path } => {
      write_u16(buf, ServerPacketId::RemoveAsset as u16);
      write_string_u16(buf, path);
    }
    ServerPacket::AssetStreamStart { name, asset } => {
      write_u16(buf, ServerPacketId::AssetStreamStart as u16);
      write_string_u16(buf, name);
      write_u64(buf, asset.last_modified);
      write_bool(buf, asset.cache_to_disk);

      let data_type_byte = match asset.data {
        AssetData::Text(_) => 0,
        AssetData::Texture(_) => 1,
        AssetData::Audio(_) => 2,
        AssetData::Data(_) => 3,
      };

      buf.push(data_type_byte);

      write_u64(buf, asset.len() as u64);
    }
    ServerPacket::AssetStream { data } => {
      write_u16(buf, ServerPacketId::AssetStream as u16);
      write_u16(buf, data.len() as u16);
      write_data(buf, data);
    }
    ServerPacket::Preload { asset_path } => {
      write_u16(buf, ServerPacketId::Preload as u16);
      write_string_u16(buf, asset_path);
    }
    ServerPacket::CustomEmotesPath { asset_path } => {
      write_u16(buf, ServerPacketId::CustomEmotesPath as u16);
      write_string_u16(buf, asset_path);
    }
    ServerPacket::MapUpdate { map_path } => {
      write_u16(buf, ServerPacketId::MapUpdate as u16);
      write_string_u16(buf, map_path);
    }
    ServerPacket::Health { health, max_health } => {
      write_u16(buf, ServerPacketId::Health as u16);
      write_u32(buf, health);
      write_u32(buf, max_health);
    }
    ServerPacket::Emotion { emotion } => {
      write_u16(buf, ServerPacketId::Emotion as u16);
      buf.push(emotion);
    }
    ServerPacket::Money { money } => {
      write_u16(buf, ServerPacketId::Money as u16);
      write_u32(buf, money);
    }
    ServerPacket::AddItem {
      id,
      name,
      description,
    } => {
      write_u16(buf, ServerPacketId::AddItem as u16);
      write_string_u8(buf, id);
      write_string_u8(buf, name);
      write_string_u16(buf, description);
    }
    ServerPacket::RemoveItem { id } => {
      write_u16(buf, ServerPacketId::RemoveItem as u16);
      write_string_u8(buf, id);
    }
    ServerPacket::PlaySound { path } => {
      write_u16(buf, ServerPacketId::PlaySound as u16);
      write_string_u16(buf, path);
    }
    ServerPacket::ExcludeObject { id } => {
      write_u16(buf, ServerPacketId::ExcludeObject as u16);
      write_u32(buf, id);
    }
    ServerPacket::IncludeObject { id } => {
      write_u16(buf, ServerPacketId::IncludeObject as u16);
      write_u32(buf, id);
    }
    ServerPacket::ExcludeActor { actor_id } => {
      write_u16(buf, ServerPacketId::ExcludeActor as u16);
      write_string_u16(buf, actor_id);
    }
    ServerPacket::IncludeActor { actor_id } => {
      write_u16(buf, ServerPacketId::IncludeActor as u16);
      write_string_u16(buf, actor_id);
    }
    ServerPacket::MoveCamera { x, y, z, hold_time } => {
      write_u16(buf, ServerPacketId::MoveCamera as u16);
      write_f32(buf, x);
      write_f32(buf, y);
      write_f32(buf, z);
      write_f32(buf, hold_time);
    }
    ServerPacket::SlideCamera { x, y, z, duration } => {
      write_u16(buf, ServerPacketId::SlideCamera as u16);
      write_f32(buf, x);
      write_f32(buf, y);
      write_f32(buf, z);
      write_f32(buf, duration);
    }
    ServerPacket::ShakeCamera { strength, duration } => {
      write_u16(buf, ServerPacketId::ShakeCamera as u16);
      write_f32(buf, strength);
      write_f32(buf, duration);
    }
    ServerPacket::FadeCamera {
      duration,
      color: (r, g, b, a),
    } => {
      write_u16(buf, ServerPacketId::FadeCamera as u16);
      write_f32(buf, duration);
      buf.push(r);
      buf.push(g);
      buf.push(b);
      buf.push(a);
    }
    ServerPacket::TrackWithCamera { actor_id } => {
      write_u16(buf, ServerPacketId::TrackWithCamera as u16);
      write_bool(buf, actor_id.is_some());

      if let Some(actor_id) = actor_id.as_ref() {
        write_string_u16(buf, actor_id);
      }
    }
    ServerPacket::UnlockCamera => {
      write_u16(buf, ServerPacketId::UnlockCamera as u16);
    }
    ServerPacket::LockInput => {
      write_u16(buf, ServerPacketId::LockInput as u16);
    }
    ServerPacket::UnlockInput => {
      write_u16(buf, ServerPacketId::UnlockInput as u16);
    }
    ServerPacket::Teleport {
      warp,
      x,
      y,
      z,
      direction,
    } => {
      write_u16(buf, ServerPacketId::Teleport as u16);
      write_bool(buf, warp);
      write_f32(buf, x);
      write_f32(buf, y);
      write_f32(buf, z);
      buf.push(translate_direction(direction));
    }
    ServerPacket::Message {
      message,
      mug_texture_path,
      mug_animation_path,
    } => {
      write_u16(buf, ServerPacketId::Message as u16);
      write_string_u16(buf, message);
      write_string_u16(buf, mug_texture_path);
      write_string_u16(buf, mug_animation_path);
    }
    ServerPacket::Question {
      message,
      mug_texture_path,
      mug_animation_path,
    } => {
      write_u16(buf, ServerPacketId::Question as u16);
      write_string_u16(buf, message);
      write_string_u16(buf, mug_texture_path);
      write_string_u16(buf, mug_animation_path);
    }
    ServerPacket::Quiz {
      option_a,
      option_b,
      option_c,
      mug_texture_path,
      mug_animation_path,
    } => {
      write_u16(buf, ServerPacketId::Quiz as u16);
      write_string_u16(buf, option_a);
      write_string_u16(buf, option_b);
      write_string_u16(buf, option_c);
      write_string_u16(buf, mug_texture_path);
      write_string_u16(buf, mug_animation_path);
    }
    ServerPacket::Prompt {
      character_limit,
      default_text,
    } => {
      write_u16(buf, ServerPacketId::Prompt as u16);
      write_u16(buf, character_limit);
      match default_text {
        Some(value) => write_string_u16(buf, value),
        _ => buf.push(0),
      }
    }
    ServerPacket::OpenBoard {
      current_depth,
      name,
      color,
      posts,
    } => {
      write_u16(buf, ServerPacketId::OpenBoard as u16);
      buf.push(current_depth);
      write_string_u16(buf, name);
      buf.push(color.0);
      buf.push(color.1);
      buf.push(color.2);

      write_u16(buf, posts.len() as u16);

      for post in posts {
        write_string_u16(buf, &post.id);
        write_bool(buf, post.read);
        write_string_u16(buf, &post.title);
        write_string_u16(buf, &post.author);
      }
    }
    ServerPacket::PrependPosts {
      current_depth,
      reference,
      posts,
    } => {
      write_u16(buf, ServerPacketId::PrependPosts as u16);
      buf.push(current_depth);
      write_bool(buf, reference.is_some());

      if reference.is_some() {
        write_string_u16(buf, reference.as_ref().unwrap());
      }

      write_u16(buf, posts.len() as u16);

      for post in posts {
        write_string_u16(buf, &post.id);
        write_bool(buf, post.read);
        write_string_u16(buf, &post.title);
        write_string_u16(buf, &post.author);
      }
    }
    ServerPacket::AppendPosts {
      current_depth,
      reference,
      posts,
    } => {
      write_u16(buf, ServerPacketId::AppendPosts as u16);
      buf.push(current_depth);
      write_bool(buf, reference.is_some());

      if reference.is_some() {
        write_string_u16(buf, reference.as_ref().unwrap());
      }

      write_u16(buf, posts.len() as u16);

      for post in posts {
        write_string_u16(buf, &post.id);
        write_bool(buf, post.read);
        write_string_u16(buf, &post.title);
        write_string_u16(buf, &post.author);
      }
    }
    ServerPacket::RemovePost { current_depth, id } => {
      write_u16(buf, ServerPacketId::RemovePost as u16);
      buf.push(current_depth);
      write_string_u16(buf, id);
    }
    ServerPacket::PostSelectionAck => {
      write_u16(buf, ServerPacketId::PostSelectionAck as u16);
    }
    ServerPacket::CloseBBS => {
      write_u16(buf, ServerPacketId::CloseBBS as u16);
    }
    ServerPacket::ShopInventory { items } => {
      write_u16(buf, ServerPacketId::ShopInventory as u16);

      write_u16(buf, items.len() as u16);

      for item in items {
        write_string_u8(buf, &item.name);
        write_string_u8(buf, &item.description);
        write_u32(buf, item.price);
      }
    }
    ServerPacket::OpenShop {
      mug_texture_path,
      mug_animation_path,
    } => {
      write_u16(buf, ServerPacketId::OpenShop as u16);
      write_string_u16(buf, mug_texture_path);
      write_string_u16(buf, mug_animation_path);
    }
    ServerPacket::OfferPackage {
      package_info,
      package_path,
    } => {
      write_u16(buf, ServerPacketId::OfferPackage as u16);
      buf.push(package_info.category as u8);
      write_string_u8(buf, &package_info.id);
      write_string_u8(buf, &package_info.name);
      write_string_u16(buf, package_path);
    }
    ServerPacket::LoadPackage {
      package_path,
      package_category,
    } => {
      write_u16(buf, ServerPacketId::LoadPackage as u16);
      write_string_u16(buf, package_path);
      buf.push(package_category as u8)
    }
    ServerPacket::ModWhitelist { whitelist_path } => {
      write_u16(buf, ServerPacketId::ModWhitelist as u16);
      write_string_u16(buf, whitelist_path);
    }
    ServerPacket::InitiateEncounter {
      package_path,
      data_path,
    } => {
      write_u16(buf, ServerPacketId::InitiateEncounter as u16);
      write_string_u16(buf, package_path);
      write_string_u16(buf, data_path);
    }
    ServerPacket::InitiatePvp { address } => {
      write_u16(buf, ServerPacketId::InitiatePvp as u16);
      write_string_u16(buf, address);
    }
    ServerPacket::ActorConnected {
      ticket,
      name,
      texture_path,
      animation_path,
      direction,
      x,
      y,
      z,
      solid,
      warp_in,
      scale_x,
      scale_y,
      rotation,
      minimap_color: (r, g, b, a),
      animation,
    } => {
      write_u16(buf, ServerPacketId::ActorConnected as u16);
      write_string_u16(buf, ticket);
      write_string_u16(buf, name);
      write_string_u16(buf, texture_path);
      write_string_u16(buf, animation_path);
      buf.push(translate_direction(direction));
      write_f32(buf, x);
      write_f32(buf, y);
      write_f32(buf, z);
      write_bool(buf, solid);
      write_bool(buf, warp_in);
      write_f32(buf, scale_x);
      write_f32(buf, scale_y);
      write_f32(buf, rotation);

      buf.push(r);
      buf.push(g);
      buf.push(b);
      buf.push(a);

      write_bool(buf, animation.is_some());

      if let Some(animation) = animation {
        write_string_u16(buf, animation);
      }
    }
    ServerPacket::ActorDisconnected { ticket, warp_out } => {
      write_u16(buf, ServerPacketId::ActorDisconnected as u16);
      write_string_u16(buf, ticket);
      write_bool(buf, warp_out);
    }
    ServerPacket::ActorSetName { ticket, name } => {
      write_u16(buf, ServerPacketId::ActorSetName as u16);
      write_string_u16(buf, ticket);
      write_string_u16(buf, name);
    }
    ServerPacket::ActorMove {
      ticket,
      x,
      y,
      z,
      direction,
    } => {
      write_u16(buf, ServerPacketId::ActorMove as u16);
      write_string_u16(buf, ticket);
      write_f32(buf, x);
      write_f32(buf, y);
      write_f32(buf, z);
      buf.push(translate_direction(direction));
    }
    ServerPacket::ActorSetAvatar {
      ticket,
      texture_path,
      animation_path,
    } => {
      write_u16(buf, ServerPacketId::ActorSetAvatar as u16);
      write_string_u16(buf, ticket);
      write_string_u16(buf, texture_path);
      write_string_u16(buf, animation_path);
    }
    ServerPacket::ActorEmote {
      ticket,
      emote_id,
      use_custom_emotes,
    } => {
      write_u16(buf, ServerPacketId::ActorEmote as u16);
      write_string_u16(buf, ticket);
      buf.push(emote_id);
      write_bool(buf, use_custom_emotes);
    }
    ServerPacket::ActorAnimate {
      ticket,
      state,
      loop_animation,
    } => {
      write_u16(buf, ServerPacketId::ActorAnimate as u16);
      write_string_u16(buf, ticket);
      write_string_u16(buf, state);
      write_bool(buf, loop_animation)
    }
    ServerPacket::ActorPropertyKeyFrames {
      ticket,
      tail,
      keyframes,
    } => {
      write_u16(buf, ServerPacketId::ActorPropertyKeyFrames as u16);
      write_string_u16(buf, ticket);
      write_bool(buf, tail);
      write_u16(buf, keyframes.len() as u16);

      for keyframe in keyframes {
        write_f32(buf, keyframe.duration);
        write_u16(buf, keyframe.property_steps.len() as u16);

        for (property, ease) in &keyframe.property_steps {
          buf.push(get_ease_identifier(ease));
          buf.push(get_actor_property_identifier(property));

          match property {
            ActorProperty::Animation(value) => write_string_u16(buf, value),
            ActorProperty::AnimationSpeed(value) => write_f32(buf, *value),
            ActorProperty::X(value) => write_f32(buf, *value),
            ActorProperty::Y(value) => write_f32(buf, *value),
            ActorProperty::Z(value) => write_f32(buf, *value),
            ActorProperty::ScaleX(value) => write_f32(buf, *value),
            ActorProperty::ScaleY(value) => write_f32(buf, *value),
            ActorProperty::Rotation(value) => write_f32(buf, *value),
            ActorProperty::Direction(value) => buf.push(translate_direction(*value)),
            ActorProperty::SoundEffect(value) => write_string_u16(buf, value),
            ActorProperty::SoundEffectLoop(value) => write_string_u16(buf, value),
          }
        }
      }
    }
    ServerPacket::ActorMinimapColor {
      ticket,
      color: (r, g, b, a),
    } => {
      write_u16(buf, ServerPacketId::ActorMinimapColor as u16);
      write_string_u16(buf, ticket);
      buf.push(r);
      buf.push(g);
      buf.push(b);
      buf.push(a);
    }
  }

  vec
}

pub fn create_asset_stream<'a>(
  max_payload_size: usize,
  name: &'a str,
  asset: &'a Asset,
) -> Vec<ServerPacket<'a>> {
  // reliability type + id + packet type + data size
  const HEADER_SIZE: usize = 1 + 8 + 2 + 2 + 16;

  let mut packets = vec![ServerPacket::AssetStreamStart { name, asset }];

  let mut bytes = match &asset.data {
    AssetData::Text(data) => data.as_bytes(),
    AssetData::Texture(data) => data,
    AssetData::Audio(data) => data,
    AssetData::Data(data) => data,
  };

  let mut remaining_bytes = bytes.len();

  while remaining_bytes > 0 {
    let available_room = max_payload_size - HEADER_SIZE;
    let size = if remaining_bytes < available_room {
      remaining_bytes
    } else {
      available_room
    };

    packets.push(ServerPacket::AssetStream {
      data: &bytes[..size],
    });

    bytes = &bytes[size..];

    remaining_bytes -= size;
  }

  packets
}

fn translate_direction(direction: Direction) -> u8 {
  match direction {
    Direction::Up => 0x01,
    Direction::Left => 0x02,
    Direction::Down => 0x04,
    Direction::Right => 0x08,
    Direction::UpLeft => 0x10,
    Direction::UpRight => 0x20,
    Direction::DownLeft => 0x40,
    Direction::DownRight => 0x80,
    _ => 0x00,
  }
}

fn get_actor_property_identifier(property: &ActorProperty) -> u8 {
  match property {
    ActorProperty::Animation(_) => 0,
    ActorProperty::AnimationSpeed(_) => 1,
    ActorProperty::X(_) => 2,
    ActorProperty::Y(_) => 3,
    ActorProperty::Z(_) => 4,
    ActorProperty::ScaleX(_) => 5,
    ActorProperty::ScaleY(_) => 6,
    ActorProperty::Rotation(_) => 7,
    ActorProperty::Direction(_) => 8,
    ActorProperty::SoundEffect(_) => 9,
    ActorProperty::SoundEffectLoop(_) => 10,
  }
}

fn get_ease_identifier(ease: &Ease) -> u8 {
  match ease {
    Ease::Linear => 0,
    Ease::In => 1,
    Ease::Out => 2,
    Ease::InOut => 3,
    Ease::Floor => 4,
  }
}
