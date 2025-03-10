use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

pub struct DebugMaterialsPlugin;

impl Plugin for DebugMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<DebugNormalsMaterial>::default())
            .add_plugins(MaterialPlugin::<DebugUVsMaterial>::default());
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct DebugNormalsMaterial {}

impl Material for DebugNormalsMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/debug_normals.wgsl".into()
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct DebugUVsMaterial {}

impl Material for DebugUVsMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/debug_uvs.wgsl".into()
    }
}
