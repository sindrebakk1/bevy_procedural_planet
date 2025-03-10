use avian3d::{math::Scalar, prelude::*};
use bevy::{
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

pub mod character_controller;
pub mod gravity;
mod integrator;

pub use character_controller::CharacterController;
pub use gravity::{GlobalGravity, GravityField, LocalGravity};

use character_controller::CharacterControllerPlugin;
use gravity::GravityPlugin;
use integrator::CustomIntegratorPlugin;

pub struct PhysicsPlugin {
    schedule: Interned<dyn ScheduleLabel>,
    length_unit: Scalar,
}

impl PhysicsPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            length_unit: 1.0,
        }
    }

    pub fn with_length_unit(mut self, unit: Scalar) -> Self {
        self.length_unit = unit;
        self
    }
}

impl Default for PhysicsPlugin {
    fn default() -> Self {
        Self::new(FixedPostUpdate)
    }
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            PhysicsPlugins::new(self.schedule)
                .with_length_unit(self.length_unit)
                .build()
                .disable::<IntegratorPlugin>()
                .add_after::<PhysicsSchedulePlugin>(CustomIntegratorPlugin::default()),
        )
        .add_plugins(GravityPlugin)
        .add_plugins(CharacterControllerPlugin)
        .insert_resource(Time::from_hz(144.0));
    }
}
