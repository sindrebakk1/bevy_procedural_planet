pub mod integrator;
pub mod helpers;
pub mod gravity;

use avian3d::math::{Scalar, Vector};
use avian3d::prelude::mass_properties::components::GlobalAngularInertia;
use avian3d::prelude::*;
use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use integrator::CustomIntegratorPlugin;

pub use gravity::LocalGravity;

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
                .add(CustomIntegratorPlugin::default()),
        )
        .insert_resource(Time::from_hz(144.0));
    }
}
