use avian3d::math::AdjustPrecision;
use avian3d::prelude::*;
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
};
use bevy_tnua::TnuaUserControlsSystemSet;
use big_space::prelude::FloatingOrigin;

pub mod controls;

pub use controls::PlayerCamera;

use crate::plugins::{physics::CharacterController, terrain::GenerateMeshes};
use controls::{
    apply_camera_controls, apply_player_controls, grab_ungrab_mouse, ForwardFromCamera,
};

#[derive(Component, Default)]
#[require(Transform, CharacterController, ForwardFromCamera, Name(|| Name::new("Player")))]
#[component(on_add = on_add_player)]
pub struct Player;

fn on_add_player(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let mesh_handle = world
        .get_resource_mut::<Assets<Mesh>>()
        .unwrap()
        .add(Capsule3d::new(0.5, 1.0));
    let material_handle = world
        .get_resource_mut::<Assets<StandardMaterial>>()
        .unwrap()
        .add(Color::srgb(0.8, 0.7, 0.6));

    let spawn_position = world
        .entity(entity)
        .get::<Transform>()
        .expect("expected entity to have Transform component")
        .translation;

    world
        .commands()
        .entity(entity)
        .insert((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            ColliderConstructor::TrimeshFromMesh,
            FloatingOrigin,
        ))
        .trigger(GenerateMeshes(spawn_position.adjust_precision()));

    world.commands().spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_translation(spawn_position - (Vec3::NEG_Z * -10.0))
            .looking_at(Vec3::NEG_Z, Vec3::Y),
    ));
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, grab_ungrab_mouse)
            .add_systems(
                PostUpdate,
                apply_camera_controls
                    .after(PhysicsSet::Sync)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_systems(
                PhysicsSchedule,
                (apply_player_controls.in_set(TnuaUserControlsSystemSet),),
            );
    }
}
