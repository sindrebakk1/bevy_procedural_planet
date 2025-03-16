use avian3d::math::{Scalar, Vector};
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
    utils::HashMap,
};
use bevy_inspector_egui::inspector_options::{InspectorOptions, ReflectInspectorOptions};
use std::convert::Into;

use super::{
    cube_tree::{Axis, CubeTree},
    material::{TerrainMaterial, TerrainMaterials},
    GenerateMeshes,
};
use crate::{
    constants::physics::{EARTH_DIAMETER_M, EARTH_MASS_KG, MOON_DIAMETER_M, MOON_MASS_KG},
    math::Rectangle,
    plugins::physics::GravityField,
};

#[derive(Clone, PartialEq, Debug)]
pub struct BodyPreset {
    pub mass: Scalar,
    pub radius: Scalar,
    pub name: Option<&'static str>,
}

impl BodyPreset {
    pub const EARTH: Self = Self {
        mass: EARTH_MASS_KG,
        radius: EARTH_DIAMETER_M / 2.0,
        name: Some("Earth"),
    };

    pub const MOON: Self = Self {
        mass: MOON_MASS_KG,
        radius: MOON_DIAMETER_M / 2.0,
        name: Some("Moon"),
    };
}

impl std::ops::Div<Scalar> for BodyPreset {
    type Output = Self;

    fn div(self, rhs: Scalar) -> Self::Output {
        let mut res = self;
        res.mass /= rhs.powi(3);
        res.radius /= rhs;
        res
    }
}

#[derive(Component, Reflect, Copy, Clone, Debug, InspectorOptions)]
#[reflect(Component, InspectorOptions)]
#[require(Visibility, Transform, ChunkCache)]
#[component(on_add = on_add_body)]
pub struct Body {
    pub mass: Scalar,
    pub radius: Scalar,
    pub name: Option<&'static str>,
}

impl Body {
    pub fn new(diameter: Scalar, mass: Scalar) -> Self {
        let radius = diameter / 2.0;
        Self {
            mass,
            radius,
            name: None,
        }
    }

    pub fn from_preset(preset: BodyPreset) -> Self {
        Self {
            mass: preset.mass,
            radius: preset.radius,
            name: preset.name,
        }
    }

    fn name(&self) -> Name {
        self.name.map_or(Name::new("Body"), Name::new)
    }
}
fn on_add_body(mut world: DeferredWorld, entity: Entity, id: ComponentId) {
    let material_handle = world
        .get_resource::<TerrainMaterials>()
        .expect("expected TerrainMaterials resource to exist")
        .standard
        .clone();

    debug_assert!(world.get_by_id(entity, id).is_some());

    let body = unsafe {
        *world
            .get_by_id(entity, id)
            .unwrap_unchecked()
            .deref::<Body>()
    };
    #[cfg(debug_assertions)]
    world
        .commands()
        .entity(entity)
        .insert((
            body.name(),
            TerrainMaterial::Standard(material_handle),
            CubeTree::new(body.radius),
            GravityField::radial_from_mass(body.mass),
            Radius(body.radius),
        ))
        .trigger(GenerateMeshes(Vector::MAX));

    #[cfg(not(debug_assertions))]
    world
        .commands()
        .entity(entity)
        .insert(TerrainMaterial(material_handle))
        .trigger(crate::plugins::terrain::GenerateMeshes(Vector::MAX));
}

impl Default for Body {
    fn default() -> Self {
        Self::from_preset(BodyPreset::EARTH)
    }
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Radius(pub Scalar);

#[derive(Component, Debug)]
#[require(Name(|| Name::new("Chunk")), Visibility)]
#[component(on_add = Self::on_add)]
pub struct Chunk;

impl Chunk {
    fn on_add(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
        let parent_entity = world
            .get::<Parent>(entity)
            .expect("expected entity to have Parent")
            .get();

        debug_assert!(
            world.get::<TerrainMaterial>(parent_entity).is_some(),
            "expected parent to have Face component"
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
}

#[derive(Copy, Clone, Debug)]
pub struct Bounds(pub Rectangle);

impl std::hash::Hash for Bounds {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for &coord in &[self.0.min.x, self.0.min.y, self.0.max.x, self.0.max.y] {
            (coord.round() as i32).hash(state);
        }
    }
    fn hash_slice<H: std::hash::Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for &bounds in data {
            bounds.hash(state)
        }
    }
}

impl PartialEq<Self> for Bounds {
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
