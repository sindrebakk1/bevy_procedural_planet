#![allow(warnings)]

use avian3d::math::{AdjustPrecision, Scalar};
use avian3d::{math::Vector, prelude::Collider};
use bevy::{
    ecs::world::CommandQueue,
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
    utils::HashSet,
};
use bevy::utils::HashMap;
use big_space::grid::Grid;
use big_space::prelude::GridCell;

pub mod body;
pub mod cube_tree;
pub mod helpers;
pub mod material;
pub mod mesh;

#[cfg(debug_assertions)]
mod debug;

pub use body::{Body, BodyPreset, Radius};

use crate::Precision;
use crate::math::Rectangle;

use body::{Chunk, ChunkCache};
use cube_tree::{ChunkData, ChunkHash, CubeTree};
use material::TerrainMaterials;
use mesh::ChunkMeshBuilder;

const CHUNK_CULLING_ANGLE: Scalar = 90.0;

#[derive(Event, Copy, Clone, Default)]
pub struct GenerateMeshes(pub Vector);

#[derive(Component)]
pub struct GenerateChunk(pub Task<CommandQueue>);

#[derive(Component)]
pub struct DespawnChunk;

#[derive(Copy, Clone, Resource)]
pub struct TerrainPluginConfig {
    position_threshold: Scalar,
}

impl Default for TerrainPluginConfig {
    fn default() -> Self {
        Self {
            position_threshold: 6.0,
        }
    }
}

#[derive(Default)]
pub struct TerrainPlugin<T: Component, const SUBDIVISIONS: usize>
where
    [(); (SUBDIVISIONS + 2).pow(2)]:,
    [(); (SUBDIVISIONS + 1).pow(2) * 6]:,
{
    cfg: TerrainPluginConfig,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component, const SUBDIVISIONS: usize> Plugin for TerrainPlugin<T, SUBDIVISIONS>
where
    [(); (SUBDIVISIONS + 2).pow(2)]:,
    [(); (SUBDIVISIONS + 1).pow(2) * 6]:,
{
    fn build(&self, app: &mut App) {
        app.insert_resource(self.cfg)
            .init_resource::<TerrainMaterials>()
            .add_observer(generate_meshes::<SUBDIVISIONS>)
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
    grid_query: Query<&Grid<Precision>>,
    config: Res<TerrainPluginConfig>,
    target_query: Query<(&GridCell<Precision>, &Transform, &Parent), With<T>>,
    mut planet_query: Query<(Entity, &GridCell<Precision>, &Transform, &mut CubeTree), With<Body>>,
    mut prev_position: Local<Vector>,
) {
    let Ok((cell, pos, parent)) = target_query.get_single() else {
        return;
    };
    let Ok(grid) = grid_query.get(parent.get()) else {
        return;
    };
    let target_position = grid.grid_position_double(cell, pos).adjust_precision();
    if target_position.distance(*prev_position) < config.position_threshold {
        return;
    }
    *prev_position = target_position;

    for (entity, cell, transform, mut cube_tree) in planet_query.iter_mut() {
        let planet_position = grid.grid_position_double(cell, transform);
        let relative_pos = target_position - planet_position;
        info_once!("relative_pos: {relative_pos:?}");
        cube_tree.insert(relative_pos);
        commands
            .entity(entity)
            .trigger(GenerateMeshes(relative_pos));
    }
}

#[allow(clippy::type_complexity)]
fn generate_meshes<const SUBDIVISIONS: usize>(
    trigger: Trigger<GenerateMeshes>,
    mut commands: Commands,
    mut planet_query: Query<
        (
            &CubeTree,
            &Grid<Precision>,
            &GridCell<Precision>,
            &Transform,
            &Radius,
            &mut ChunkCache,
        ),
        With<Body>,
    >,
) where
    [(); (SUBDIVISIONS + 2).pow(2)]:,
    [(); (SUBDIVISIONS + 1).pow(2) * 6]:,
{
    let entity = trigger.entity();
    let target_position = trigger.0;
    let thread_pool = AsyncComputeTaskPool::get();

    for (cube_tree, grid, grid_cell, transform, radius, mut chunk_cache) in planet_query.iter_mut() {
        let mesh_builder = ChunkMeshBuilder::<SUBDIVISIONS>::new(radius.0);
        let mut hash_set: HashSet<ChunkHash> = HashSet::from_iter(cube_tree.iter().map(|(_, data)| data.hash));

        for (_, entity) in chunk_cache.extract_if(|bounds, _| !hash_set.contains(bounds)) {
            commands.entity(entity).insert(DespawnChunk);
        }

        let planet_pos = (grid as &Grid<Precision>).grid_position_double(grid_cell, transform);

        info_once!("planet_pos: {planet_pos:?}");

        for (&bounds, &data) in cube_tree.iter() {
            if chunk_cache.contains_key(&data.hash) {
                continue;
            }

            let (grid_cell, translation) =
                grid.translation_to_grid(data.center - planet_pos);
            let chunk_entity = commands
                .spawn((grid_cell, Transform::from_translation(translation), Name::new(format!("{:?}", data.hash.values()))))
                .set_parent(entity)
                .insert(Chunk)
                .id();

            chunk_cache.insert(data.hash, chunk_entity);

            let has_collider = data.hash.collider();
            let task = thread_pool.spawn(async move {
                let mut command_queue = CommandQueue::default();

                let mesh = mesh_builder.build(&bounds, &data);
                let collider = has_collider.then(|| {
                    Collider::trimesh_from_mesh(&mesh)
                        .expect("expected collider construction to succeed")
                });

                command_queue.push(move |world: &mut World| {
                    let mesh_handle = world
                        .get_resource_mut::<Assets<Mesh>>()
                        .expect("expected Assets<Mesh> resource to exist")
                        .add(mesh);

                    if let Ok(mut entity_mut) = world.get_entity_mut(chunk_entity) {
                        match collider {
                            Some(collider) => {
                                entity_mut.insert((collider, Mesh3d(mesh_handle)))
                            }
                            None => entity_mut.insert(Mesh3d(mesh_handle)),
                        };
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
    mut tasks: Query<(Entity, &mut GenerateChunk), With<Chunk>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(mut commands_queue) = block_on(poll_once(&mut task.0)) {
            commands.append(&mut commands_queue);
            commands.entity(entity).remove::<GenerateChunk>();
        }
    }
}

fn handle_despawn_chunks(mut commands: Commands, mut query: Query<Entity, With<DespawnChunk>>) {
    for entity in query.iter_mut() {
        commands.entity(entity).remove_parent().despawn();
    }
}
