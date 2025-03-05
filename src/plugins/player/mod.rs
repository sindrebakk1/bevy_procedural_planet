pub mod tuna_controller;

use avian3d::collision::ColliderConstructor;
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::{
    App, Assets, Capsule3d, Color, Component, Entity, Mesh, Mesh3d, MeshMaterial3d, Name, Plugin,
    StandardMaterial, Transform,
};

use tuna_controller::{CharacterController, CharacterControllerPlugin};

#[derive(Component, Default)]
#[require(Transform, CharacterController, Name(|| Name::new("Player")))]
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

    world.commands().entity(entity).insert((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        ColliderConstructor::TrimeshFromMesh,
    ));
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CharacterControllerPlugin);
    }
}
