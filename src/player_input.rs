use std::f32::consts::TAU;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct PlayerInputPlugin;

impl Plugin for PlayerInputPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<PlayerInput>()
        ;
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Reflect)]
#[type_path = "game"]
pub enum Speed {
    #[default]
    Normal,
    Slow,
    Crawl,
    Fast,
}

#[allow(unused)]
impl Speed {
    pub fn mul(&self) -> f32 {
        match self {
            Speed::Fast => 2.0,
            Speed::Normal => 1.0,
            Speed::Slow => 0.5,
            Speed::Crawl => 0.25,
        }
    }
    pub fn faster(&self) -> Speed {
        match self {
            Speed::Fast => Speed::Fast,
            Speed::Normal => Speed::Fast,
            Speed::Slow => Speed::Normal,
            Speed::Crawl => Speed::Slow,
        }
    }
    pub fn slower(&self) -> Speed {
        match self {
            Speed::Fast => Speed::Normal,
            Speed::Normal => Speed::Slow,
            Speed::Slow => Speed::Crawl,
            Speed::Crawl => Speed::Crawl,
        }
    }
}

/// Represents a floating point value from -1.0 to 1.0 with 7 bits precision.
#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[reflect(Clone)]
#[type_path = "game"]
pub struct FloatFixedOne8(i8);

impl std::fmt::Debug for FloatFixedOne8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} ({})", Into::<f32>::into(*self), self.0)
    }
}

impl From<f32> for FloatFixedOne8 {
    fn from(value: f32) -> Self {
        let value = value.clamp(-1., 1.);
        Self((value * 127.0).floor() as i8)
    }
}

impl From<FloatFixedOne8> for f32 {
    fn from(value: FloatFixedOne8) -> f32 {
        value.0 as f32 / 127.0
    }
}

/// Instantaneous movement from a tick's worth of input.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Reflect)]
#[reflect(Clone)]
#[type_path = "game"]
pub struct PlayerMove {
    pub forward_back: FloatFixedOne8,
    pub right_left: FloatFixedOne8,
    pub up_down: FloatFixedOne8,
    pub speed: Speed,
}

impl PlayerMove {
    pub fn new(thrust: Vec3, speed: Speed) -> Self {
        Self {
            forward_back: thrust.z.into(),
            right_left: thrust.x.into(),
            up_down: thrust.y.into(),
            speed,
        }
    }
}

/// Represent the player's turn angles, in X/Y/Z.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
#[reflect(Clone)]
#[type_path = "game"]
pub struct PlayerBodyTurn(FloatFixedOne8, FloatFixedOne8, FloatFixedOne8);

impl PlayerBodyTurn {
    pub fn new(euler: Vec3) -> Self {
        let into = |r: f32| -> FloatFixedOne8 { ((r % TAU) / TAU).into() };
        Self(into(euler.x), into(euler.y), into(euler.z))
    }

    pub fn get_euler(&self) -> Vec3 {
        let to = |f: FloatFixedOne8| -> f32 { Into::<f32>::into(f) * TAU };
        let (y, x, z) : (f32, f32, f32) = (to(self.1), to(self.0), to(self.2));
        Vec3::new(x, y, z)
    }
}

/// Represent the player's look angles, in X/Y/Z.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
#[reflect(Clone)]
#[type_path = "game"]
pub struct PlayerHeadTurn(FloatFixedOne8, FloatFixedOne8, FloatFixedOne8);

impl PlayerHeadTurn {
    pub fn new(euler: Vec3) -> Self {
        let into = |r: f32| -> FloatFixedOne8 { ((r % TAU) / TAU).into() };
        Self(into(euler.x), into(euler.y), into(euler.z))
    }

    pub fn get_euler(&self) -> Vec3 {
        let to = |f: FloatFixedOne8| -> f32 { Into::<f32>::into(f) * TAU };
        let (y, x, z) : (f32, f32, f32) = (to(self.1), to(self.0), to(self.2));
        Vec3::new(x, y, z)
    }
}

/// Client input.
/// The first entry is the Player entity who generated the event.
#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
#[reflect(Clone)]
#[type_path = "game"]
pub enum PlayerInput {
    /// Player movement (relative).
    /// This is an uninterpreted result of all inputs (jump, move, strafe, etc).
    /// The server determines what actual physical movement results.
    Move(Entity, PlayerMove),
    /// Player body turn (relative).
    /// In an FPS context, normally only Y is edited. Mouselook goes into Look.
    BodyTurn(Entity, PlayerBodyTurn),
    /// Player head turn (relative).
    /// This doesn't affect body orientation.
    HeadTurn(Entity, PlayerHeadTurn),
    /// About-face turn.
    TurnAround(Entity),
    /// Remove tilt.
    Straighten(Entity),
    /// Start holding fire button.
    StartFire(Entity),
    /// Stop holding fire button.
    StopFire(Entity),
    /// Toggle crouching.
    ToggleCrouch(Entity),
}

impl PlayerInput {
    pub fn player_entity(&self) -> Entity {
        match self {
            PlayerInput::Move(entity, _) |
            PlayerInput::BodyTurn(entity, _) |
            PlayerInput::HeadTurn(entity, _) |
            PlayerInput::TurnAround(entity) |
            PlayerInput::Straighten(entity) |
            PlayerInput::StartFire(entity) |
            PlayerInput::StopFire(entity) |
            PlayerInput::ToggleCrouch(entity) => *entity
        }
    }
}
