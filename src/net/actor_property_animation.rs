use super::Direction;

#[derive(Debug)]
pub enum ActorProperty {
  Animation(String),
  AnimationSpeed(f32),
  X(f32),
  Y(f32),
  Z(f32),
  ScaleX(f32),
  ScaleY(f32),
  Rotation(f32),
  Direction(Direction),
  SoundEffect(String),
  SoundEffectLoop(String),
}

#[derive(Debug)]
pub enum Ease {
  Linear,
  In,
  Out,
  InOut,
  Floor,
}

#[derive(Debug)]
pub struct KeyFrame {
  pub property_steps: Vec<(ActorProperty, Ease)>,
  pub duration: f32,
}
