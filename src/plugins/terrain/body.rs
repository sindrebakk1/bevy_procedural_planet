use super::cube_tree::{Axis, CubeTree};
use crate::plugins::terrain::material::{TerrainMaterial, TerrainMaterials};

use crate::constants::physics::{EARTH_DIAMETER_M, EARTH_MASS_KG, MOON_DIAMETER_M, MOON_MASS_KG};
use bevy::core::Name;
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::{CommandQueue, DeferredWorld};
use bevy::hierarchy::Parent;
use bevy::math::{Rect, Vec3};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::{Component, Entity, Event, Reflect, ReflectComponent, Transform, Visibility};
use bevy::tasks::Task;
use bevy::utils::HashMap;
use bevy_inspector_egui::inspector_options::ReflectInspectorOptions;
use bevy_inspector_egui::InspectorOptions;
use std::hash::{Hash, Hasher};
use std::ops::Div;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BodyPreset {
    pub mass: f32,
    pub radius: f32,
}

impl BodyPreset {
    pub const EARTH: Self = Self {
        mass: EARTH_MASS_KG,
        radius: EARTH_DIAMETER_M / 2.0,
    };

    pub const MOON: Self = Self {
        mass: MOON_MASS_KG,
        radius: MOON_DIAMETER_M / 2.0,
    };
}

impl Div<f32> for BodyPreset {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        let mut res = self;
        res.mass /= rhs;
        res.radius /= rhs;
        res
    }
}

#[derive(Event, Copy, Clone, Default)]
pub struct GenerateMeshes(pub Vec3);

#[derive(Component, Reflect, Clone, Debug, InspectorOptions)]
#[reflect(Component, InspectorOptions)]
#[require(Visibility, Transform, ChunkCache, Name(|| Name::new("Body")))]
#[component(on_add = on_add_body)]
pub struct Body {
    pub mass: f32,
    pub radius: f32,
}

impl Body {
    pub fn new(diameter: f32, mass: f32) -> Self {
        let radius = diameter / 2.0;
        Self { mass, radius }
    }

    pub fn from_preset(preset: BodyPreset) -> Self {
        Self {
            mass: preset.mass,
            radius: preset.radius,
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::from_preset(BodyPreset::EARTH)
    }
}

fn on_add_body(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let material_handle = world
        .get_resource::<TerrainMaterials>()
        .expect("expected TerrainMaterials resource to exist")
        .standard
        .clone();

    let planet = world
        .get::<Body>(entity)
        .expect("expected Body component to exist")
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
        .insert((CubeTree::new(planet.radius), Radius(planet.radius)))
        .trigger(GenerateMeshes(Vec3::MAX));
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Radius(pub f32);

#[derive(Component, Debug)]
#[require(Name(|| Name::new("Chunk")))]
#[component(on_add = on_add_chunk)]
pub struct Chunk;

fn on_add_chunk(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let parent_entity = world
        .get::<Parent>(entity)
        .expect("expected entity to have Parent")
        .get();

    debug_assert!(
        world.get::<Body>(parent_entity).is_some(),
        "expected parent to have Body component"
    );
    debug_assert!(
        world.get::<TerrainMaterial>(parent_entity).is_some(),
        "expected parent to have TerrainMaterial component"
    );

    let material = unsafe {
        world
            .get::<TerrainMaterial>(parent_entity)
            .unwrap_unchecked()
            .clone()
    };

    #[cfg(debug_assertions)]
    match material {
        TerrainMaterial::DebugNormals(handle) => {
            world
                .commands()
                .entity(entity)
                .insert(MeshMaterial3d(handle.clone()));
        }
        TerrainMaterial::DebugUVs(handle) => {
            world
                .commands()
                .entity(entity)
                .insert(MeshMaterial3d(handle.clone()));
        }
        TerrainMaterial::Standard(handle) => {
            world
                .commands()
                .entity(entity)
                .insert(MeshMaterial3d(handle.clone()));
        }
    };

    #[cfg(not(debug_assertions))]
    world
        .commands()
        .entity(entity)
        .insert(MeshMaterial3d(material.handle()));
}

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
