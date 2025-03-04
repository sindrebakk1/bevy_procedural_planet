mod controller;

use bevy::prelude::{Component, Name, Transform};
use controller::CharacterController;

#[derive(Component, Default)]
#[require(Transform, CharacterController, Name(|| Name::new("Player")))]
pub struct Player;
