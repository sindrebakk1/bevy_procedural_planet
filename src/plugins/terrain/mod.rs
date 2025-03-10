use avian3d::{math::Vector, prelude::Collider};
use bevy::{
    ecs::world::CommandQueue,
    math::Vec3,
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
    utils::HashSet,
};

pub mod body;
pub mod cube_tree;
pub mod helpers;
pub mod material;
pub mod mesh;

#[cfg(debug_assertions)]
mod debug;

pub use body::{Body, BodyPreset, Radius};

use body::{Bounds, Chunk, ChunkCache};
use cube_tree::{CubeTree, CubeTreeNode};
use material::TerrainMaterials;
use mesh::ChunkMeshBuilder;

const CHUNK_CULLING_ANGLE: f32 = 90.0;

#[derive(Event, Copy, Clone, Default)]
pub struct GenerateMeshes(pub Vector);

#[derive(Component)]
pub struct GenerateChunk(pub Task<CommandQueue>);

#[derive(Component)]
pub struct DespawnChunk;

#[derive(Copy, Clone, Resource)]
pub struct TerrainPluginConfig {
    position_threshold: f32,
}

impl Default for TerrainPluginConfig {
    fn default() -> Self {
        Self {
            position_threshold: 6.0,
        }
    }
}

#[derive(Default)]
pub struct TerrainPlugin<T: Component> {
    cfg: TerrainPluginConfig,
    _marker: std::marker::PhantomData<T>,
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
    mut planet_query: Query<(Entity, &Transform, &mut CubeTree), With<Body>>,
    mut prev_position: Local<Vec3>,
) {
    let target_translation = transform_query.single().translation;
    if target_translation.distance(*prev_position) < config.position_threshold {
        return;
    }
    *prev_position = target_translation;

    for (entity, transform, mut cube_tree) in planet_query.iter_mut() {
        let relative_pos = target_translation - transform.translation;
        cube_tree.insert(relative_pos);
        commands
            .entity(entity)
            .trigger(GenerateMeshes(target_translation));
    }
}

fn generate_meshes(
    trigger: Trigger<GenerateMeshes>,
    mut commands: Commands,
    mut planet_query: Query<(&CubeTree, &Radius, &mut ChunkCache), With<Body>>,
) {
    let entity = trigger.entity();
    let target_position = trigger.0;
    let thread_pool = AsyncComputeTaskPool::get();

    for (cube_tree, radius, mut cache) in planet_query.iter_mut() {
        for (axis, root_node) in cube_tree.faces.iter() {
            let mesh_builder = ChunkMeshBuilder::new(radius.0);
            let chunk_cache = cache.get_mut(axis).unwrap();

            let children = root_node.filtered_children(|node: &CubeTreeNode| {
                node.center().map_or(false, |center| {
                    (center - target_position)
                        .normalize()
                        .dot(center.normalize())
                        > CHUNK_CULLING_ANGLE.to_radians().cos()
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

                let chunk_entity = commands.spawn_empty().set_parent(entity).insert(Chunk).id();
                chunk_cache.insert(Bounds(bounds), chunk_entity);

                let has_collider = node.collider();

                let task = thread_pool.spawn(async move {
                    let mut command_queue = CommandQueue::default();

                    let mesh = mesh_builder.build(bounds, axis);
                    // let collider = Collider::trimesh_from_mesh(&mesh)
                    //     .expect("expected collider construction to succeed");
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
                            entity_mut.insert(Mesh3d(mesh_handle));
                            // entity_mut.insert((Mesh3d(mesh_handle), collider));
                            if let Some(collider) = collider {
                                info!("inserting collider");
                                entity_mut.insert(collider);
                            }
                        }
                    });
                    command_queue
                });
                commands.entity(chunk_entity).insert(GenerateChunk(task));
            }
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
