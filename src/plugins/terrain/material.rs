use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
};

use super::body::Chunk;

#[cfg(debug_assertions)]
use crate::materials::debug::{DebugNormalsMaterial, DebugUVsMaterial};

#[derive(Resource, Clone, Debug)]
pub struct TerrainMaterials {
    pub standard: Handle<StandardMaterial>,
    #[cfg(debug_assertions)]
    pub debug_normals: Handle<DebugNormalsMaterial>,
    #[cfg(debug_assertions)]
    pub debug_uvs: Handle<DebugUVsMaterial>,
}

impl FromWorld for TerrainMaterials {
    fn from_world(world: &mut World) -> Self {
        let standard_handle = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .expect("Expected Assets<StandardMaterial> to exist")
            .add(StandardMaterial {
                base_color: Color::linear_rgb(0.12, 0.64, 0.14),
                ..Default::default()
            });

        #[cfg(debug_assertions)]
        let (debug_normals_handle, debug_uvs_handle) = (
            world
                .get_resource_mut::<Assets<DebugNormalsMaterial>>()
                .expect("Expected Assets<DebugNormalsMaterial> to exist")
                .add(DebugNormalsMaterial {}),
            world
                .get_resource_mut::<Assets<DebugUVsMaterial>>()
                .expect("Expected Assets<DebugUVsMaterial> to exist")
                .add(DebugUVsMaterial {}),
        );

        Self {
            standard: standard_handle,
            #[cfg(debug_assertions)]
            debug_normals: debug_normals_handle,
            #[cfg(debug_assertions)]
            debug_uvs: debug_uvs_handle,
        }
    }
}

#[cfg(debug_assertions)]
#[derive(Component, Clone, Eq, PartialEq, Debug)]
#[component(on_insert = on_insert_terrain_material)]
pub enum TerrainMaterial {
    DebugNormals(Handle<DebugNormalsMaterial>),
    DebugUVs(Handle<DebugUVsMaterial>),
    Standard(Handle<StandardMaterial>),
}

#[cfg(not(debug_assertions))]
#[derive(Component, Clone, Eq, PartialEq, Debug)]
#[component(on_insert = on_insert_terrain_material)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

#[cfg(not(debug_assertions))]
impl TerrainMaterial {
    pub fn handle(&self) -> Handle<StandardMaterial> {
        self.0.clone()
    }
}

fn on_insert_terrain_material(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let terrain_material = world
        .get::<TerrainMaterial>(entity)
        .expect("expected entity to have TerrainMaterial component")
        .clone();

    let Some(children) = world.get::<Children>(entity) else {
        return;
    };
    let children: Vec<Entity> = children.iter().copied().collect();

    for &child_entity in children.iter() {
        if world.entity(child_entity).contains::<Chunk>() {
            #[cfg(debug_assertions)]
            match terrain_material.clone() {
                TerrainMaterial::Standard(handle) => world
                    .commands()
                    .entity(child_entity)
                    .remove::<MeshMaterial3d<DebugNormalsMaterial>>()
                    .remove::<MeshMaterial3d<DebugUVsMaterial>>()
                    .insert(MeshMaterial3d::<StandardMaterial>(handle.clone())),

                TerrainMaterial::DebugNormals(handle) => world
                    .commands()
                    .entity(child_entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .remove::<MeshMaterial3d<DebugUVsMaterial>>()
                    .insert(MeshMaterial3d::<DebugNormalsMaterial>(handle.clone())),

                TerrainMaterial::DebugUVs(handle) => world
                    .commands()
                    .entity(child_entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .remove::<MeshMaterial3d<DebugNormalsMaterial>>()
                    .insert(MeshMaterial3d::<DebugUVsMaterial>(handle.clone())),
            };
            #[cfg(not(debug_assertions))]
            world
                .commands()
                .entity(child_entity)
                .insert(MeshMaterial3d::<StandardMaterial>(
                    terrain_material.handle(),
                ));
        }
    }
}
