pub mod cube_tree;
pub mod helpers;
pub mod material;
pub mod mesh;
pub mod planet;

#[cfg(debug_assertions)]
mod debug;

use crate::plugins::terrain::cube_tree::CubeTreeNode;
use crate::plugins::terrain::material::{TerrainMaterial, TerrainMaterials};
use crate::plugins::terrain::mesh::ChunkMeshBuilder;
use crate::plugins::terrain::planet::{
    Bounds, Chunk, ChunkCache, DespawnChunk, GenerateChunk, GenerateMeshes, Planet,
};
use avian3d::collision::ColliderConstructor;
use bevy::app::{App, Plugin, Update};
use bevy::asset::Assets;
use bevy::ecs::world::CommandQueue;
use bevy::hierarchy::{BuildChildren, Parent};
use bevy::math::Vec3;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::{
    Commands, Component, Entity, Local, Mesh, Mesh3d, Query, Res, Resource, Transform, Trigger,
    With, World,
};
use bevy::tasks::{block_on, poll_once, AsyncComputeTaskPool};
use bevy::utils::HashSet;
use std::marker::PhantomData;

const CHUNK_CULLING_DISTANCE: f32 = 500_000.0;
const CHUNK_CULLING_ANGLE: f32 = -0.8660254;

#[derive(Copy, Clone, Resource)]
pub struct TerrainPluginConfig {
    position_threshold: f32,
}

impl Default for TerrainPluginConfig {
    fn default() -> Self {
        Self {
            position_threshold: 10.0,
        }
    }
}

#[derive(Default)]
pub struct TerrainPlugin<T: Component> {
    cfg: TerrainPluginConfig,
    _marker: PhantomData<T>,
}

impl<T: Component> Plugin for TerrainPlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.cfg)
            .init_resource::<TerrainMaterials>()
            .add_observer(generate_meshes)
            .add_systems(
                Update,
                (
                    handle_chunk_generation_tasks,
                    handle_despawn_chunks,
                    track_target_position::<T>,
                ),
            );

        #[cfg(debug_assertions)]
        {
            use debug::DebugTerrainPlugin;
            app.add_plugins(DebugTerrainPlugin);
        }
    }
}

fn track_target_position<T: Component>(
    mut commands: Commands,
    config: Res<TerrainPluginConfig>,
    transform_query: Query<&Transform, With<T>>,
    mut planet_query: Query<(Entity, &mut Planet, &Transform)>,
    mut prev_position: Local<Vec3>,
) {
    let target_translation = transform_query.single().translation;
    if target_translation.distance(*prev_position) < config.position_threshold {
        return;
    }
    *prev_position = target_translation;

    for (entity, mut planet, transform) in planet_query.iter_mut() {
        let relative_pos = target_translation - transform.translation;
        planet.quad_tree.insert(relative_pos);
        commands
            .entity(entity)
            .trigger(GenerateMeshes(target_translation));
    }
}

fn generate_meshes(
    trigger: Trigger<GenerateMeshes>,
    mut commands: Commands,
    mut planet_query: Query<(&Planet, &mut ChunkCache)>,
) {
    let entity = trigger.entity();
    let target_position = trigger.0;
    let (planet, mut cache) = match planet_query.get_mut(entity) {
        Ok((planet, cache)) => (planet, cache),
        Err(_) => return,
    };

    let thread_pool = AsyncComputeTaskPool::get();

    for (axis, root_node) in planet.quad_tree.faces.iter() {
        let mesh_builder = ChunkMeshBuilder::new(planet.radius);
        let chunk_cache = cache.get_mut(axis).unwrap();

        let children = root_node.filtered_children(|node: &CubeTreeNode| {
            target_position.distance(node.center().unwrap_or(Vec3::MAX)) <= CHUNK_CULLING_DISTANCE
                && node.normal().map_or(false, |normal| {
                    normal.dot(target_position.normalize()) >= CHUNK_CULLING_ANGLE
                })
        });

        let hash_set: HashSet<_> = children.iter().map(|node| Bounds(node.bounds())).collect();
        for (_, entity) in chunk_cache.extract_if(|bounds, _| !hash_set.contains(bounds)) {
            commands.entity(entity).insert(DespawnChunk);
        }

        let axis = *axis;
        for node in children.iter() {
            let bounds = node.bounds();
            if chunk_cache.contains_key(&Bounds(bounds)) {
                continue;
            }
            let chunk_entity = commands.spawn(Chunk).set_parent(entity).id();
            chunk_cache.insert(Bounds(bounds), chunk_entity);

            let insert_collider = node.collider();

            let task = thread_pool.spawn(async move {
                let mut command_queue = CommandQueue::default();

                let mesh = mesh_builder.build(bounds, axis);

                command_queue.push(move |world: &mut World| {
                    let mesh_handle = world
                        .get_resource_mut::<Assets<Mesh>>()
                        .expect("expected Assets<Mesh> resource to exist")
                        .add(mesh);

                    if let Ok(mut entity_mut) = world.get_entity_mut(chunk_entity) {
                        entity_mut.insert(Mesh3d(mesh_handle));
                        if insert_collider {
                            entity_mut.insert(ColliderConstructor::TrimeshFromMesh);
                        }
                    }
                });
                command_queue
            });

            commands.entity(chunk_entity).insert(GenerateChunk(task));
        }
    }
}

fn handle_chunk_generation_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &Parent, &mut GenerateChunk), With<Chunk>>,
    material_query: Query<&TerrainMaterial, With<Planet>>,
) {
    for (entity, parent_entity, mut task) in tasks.iter_mut() {
        if let Some(mut commands_queue) = block_on(poll_once(&mut task.0)) {
            let material = material_query
                .get(parent_entity.get())
                .expect("expected parent entity to contain TerrainMaterial component");

            #[cfg(debug_assertions)]
            {
                let mut entity_commands = commands.entity(entity);

                match material {
                    TerrainMaterial::DebugNormals(handle) => {
                        entity_commands.insert(MeshMaterial3d(handle.clone()))
                    }
                    TerrainMaterial::DebugUVs(handle) => {
                        entity_commands.insert(MeshMaterial3d(handle.clone()))
                    }
                    TerrainMaterial::Standard(handle) => {
                        entity_commands.insert(MeshMaterial3d(handle.clone()))
                    }
                }
                .remove::<GenerateChunk>();
            }

            #[cfg(not(debug_assertions))]
            commands
                .entity(entity)
                .insert(MeshMaterial3d(material.handle()))
                .remove::<GenerateChunk>();

            commands.append(&mut commands_queue);
        }
    }
}

fn handle_despawn_chunks(mut commands: Commands, mut query: Query<Entity, With<DespawnChunk>>) {
    for entity in query.iter_mut() {
        commands.entity(entity).remove_parent().despawn();
    }
}
