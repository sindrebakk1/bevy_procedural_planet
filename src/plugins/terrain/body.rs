use super::{
    cube_tree::{Axis, CubeTree},
    material::{TerrainMaterial, TerrainMaterials},
    GenerateMeshes,
};
use crate::plugins::terrain::cube_tree::ChunkHash;
use crate::{
    constants::physics::{EARTH_DIAMETER_M, EARTH_MASS_KG, MOON_DIAMETER_M, MOON_MASS_KG},
    math::Rectangle,
    plugins::physics::GravityField,
};
use avian3d::math::{Scalar, Vector};
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
    utils::HashMap,
};
use bevy_inspector_egui::inspector_options::{InspectorOptions, ReflectInspectorOptions};
use std::convert::Into;
use std::ops::{Deref, DerefMut};

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

impl Deref for Radius {
    type Target = Scalar;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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

#[derive(Component)]
pub struct ChunkCache(HashMap<ChunkHash, Entity>);

impl Default for ChunkCache {
    fn default() -> Self {
        ChunkCache(HashMap::with_capacity(512))
    }
}

// Implement Deref to allow immutable access
impl Deref for ChunkCache {
    type Target = HashMap<ChunkHash, Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Implement DerefMut to allow mutable access
impl DerefMut for ChunkCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
