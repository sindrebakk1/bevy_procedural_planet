use bevy::prelude::Component;

pub const EARTH_GRAVITY: f32 = 9.81;

#[derive(Component, Default)]
pub struct GravitationalBody;
