use avian3d::PhysicsPlugins;
use bevy::app::{App, Plugin};
use bevy::prelude::Time;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .insert_resource(Time::from_hz(144.0));
    }
}
