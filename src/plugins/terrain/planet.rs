use super::cube_tree::{Axis, CubeTree};
use crate::physics::GravitationalBody;
use crate::plugins::terrain::material::{TerrainMaterial, TerrainMaterials};

use bevy::core::Name;
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::{CommandQueue, DeferredWorld};
use bevy::math::{Rect, Vec3};
use bevy::prelude::{Component, Entity, Event, Transform, Visibility};
use bevy::tasks::Task;
use bevy::utils::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Event, Copy, Clone, Default)]
pub struct GenerateMeshes(pub Vec3);

#[derive(Component, Clone, Debug)]
#[require(Visibility, Transform, ChunkCache, GravitationalBody, Name(|| Name::new("Planet")))]
#[component(on_add = on_add_planet)]
pub struct Planet {
    pub radius: f32,
    pub quad_tree: CubeTree,
}

impl Planet {
    pub fn new(diameter: f32) -> Self {
        let radius = diameter / 2.0;
        Self {
            radius,
            quad_tree: CubeTree::new(radius),
        }
    }
}

fn on_add_planet(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let material_handle = world
        .get_resource::<TerrainMaterials>()
        .expect("expected TerrainMaterials resource to exist")
        .standard
        .clone();

    #[cfg(debug_assertions)]
    world
        .commands()
        .entity(entity)
        .insert(TerrainMaterial::Standard(material_handle));

    #[cfg(not(debug_assertions))]
    world
        .commands()
        .entity(entity)
        .insert(TerrainMaterial(material_handle));

    world
        .commands()
        .entity(entity)
        .trigger(GenerateMeshes(Vec3::MAX));
}

#[derive(Component)]
#[require(Name(|| Name::new("Chunk")))]
pub struct Chunk;

#[derive(Component)]
pub struct GenerateChunk(pub Task<CommandQueue>);

#[derive(Component)]
pub struct DespawnChunk;

#[derive(Copy, Clone, Debug)]
pub struct Bounds(pub Rect);

impl Hash for Bounds {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for &coord in &[self.0.min.x, self.0.min.y, self.0.max.x, self.0.max.y] {
            (coord.round() as i32).hash(state);
        }
    }
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for &bounds in data {
            bounds.hash(state)
        }
    }
}

impl PartialEq for Bounds {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        [self.0.min.x, self.0.min.y, self.0.max.x, self.0.max.y]
            .iter()
            .zip(&[other.0.min.x, other.0.min.y, other.0.max.x, other.0.max.y])
            .all(|(&lhs, &rhs)| lhs.round() as i32 == rhs.round() as i32)
    }
}

impl Eq for Bounds {}

#[derive(Component)]
pub struct ChunkCache(HashMap<Axis, HashMap<Bounds, Entity>>);

impl Default for ChunkCache {
    fn default() -> Self {
        let mut cache = HashMap::with_capacity(6);
        for axis in Axis::ALL {
            cache.insert(axis, HashMap::new());
        }
        ChunkCache(cache)
    }
}

#[allow(unused)]
impl ChunkCache {
    pub fn get(&self, key: &Axis) -> Option<&HashMap<Bounds, Entity>> {
        self.0.get(key)
    }
    pub fn get_mut(&mut self, key: &Axis) -> Option<&mut HashMap<Bounds, Entity>> {
        self.0.get_mut(key)
    }
}
