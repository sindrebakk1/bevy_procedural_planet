use bevy::prelude::*;

use super::{
    material::{TerrainMaterial, TerrainMaterials},
    Body,
};
use crate::keybinds::{TOGGLE_DEBUG_NORMALS, TOGGLE_DEBUG_UVS};

#[derive(Event, Copy, Clone, Default)]
pub struct UpdateTerrainMaterial;

pub struct DebugTerrainPlugin;

impl Plugin for DebugTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_active_material.run_if(resource_changed::<ButtonInput<KeyCode>>),
        );
    }
}

fn update_active_material(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    terrain_materials: Res<TerrainMaterials>,
    query: Query<Entity, With<Body>>,
    mut debug_normals_enabled: Local<bool>,
    mut debug_uvs_enabled: Local<bool>,
) {
    if input.just_pressed(TOGGLE_DEBUG_NORMALS) {
        if *debug_normals_enabled {
            *debug_normals_enabled = false;
            for entity in query.iter() {
                commands.entity(entity).insert(TerrainMaterial::Standard(
                    terrain_materials.standard.clone(),
                ));
            }
        } else {
            *debug_normals_enabled = true;
            *debug_uvs_enabled = false;
            for entity in query.iter() {
                commands
                    .entity(entity)
                    .insert(TerrainMaterial::DebugNormals(
                        terrain_materials.debug_normals.clone(),
                    ));
            }
        }
    }
    if input.just_pressed(TOGGLE_DEBUG_UVS) {
        if *debug_uvs_enabled {
            *debug_uvs_enabled = false;
            for entity in query.iter() {
                commands.entity(entity).insert(TerrainMaterial::Standard(
                    terrain_materials.standard.clone(),
                ));
            }
        } else {
            *debug_uvs_enabled = true;
            *debug_normals_enabled = false;
            for entity in query.iter() {
                commands.entity(entity).insert(TerrainMaterial::DebugUVs(
                    terrain_materials.debug_uvs.clone(),
                ));
            }
        }
    }
}
