#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use avian3d::math::{Scalar, Vector};
use bevy::color::palettes::basic::FUCHSIA;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use procedural_planet::plugins::terrain::cube_tree::CubeTree;
use procedural_planet::{
    materials::GlobalMaterialsPlugin,
    plugins::terrain::{cube_tree::Axis, mesh::ChunkMeshBuilder},
};

const RADIUS: Scalar = 20000.0;
const SUBDIVISIONS: usize = 5;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(GlobalMaterialsPlugin)
        .add_plugins(WireframePlugin)
        .insert_resource(WireframeConfig {
            global: true,
            default_color: Default::default(),
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos);

    app.run();
}

#[derive(Bundle)]
pub struct ChunkBundle {
    mesh_3d: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
    transform: Transform,
    aabb_gizmo: ShowAabbGizmo,
}

impl ChunkBundle {
    pub fn new(mesh: Handle<Mesh>, material: Handle<StandardMaterial>, translation: Vec3) -> Self {
        Self {
            mesh_3d: Mesh3d(mesh),
            material: MeshMaterial3d(material),
            transform: Transform::from_translation(translation),
            aabb_gizmo: ShowAabbGizmo {
                color: Some(FUCHSIA.into()),
            },
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut tree = CubeTree::new(RADIUS);
    let point = (Vector::Y + Vector::X + Vector::Z) * RADIUS * 0.56;
    tree.insert(point);
    let mesh_builder = ChunkMeshBuilder::<SUBDIVISIONS>::new(RADIUS);
    let materials = Axis::ALL.map(|axis| {
        #[cfg(feature = "f64")]
        let material = StandardMaterial::from_color(Color::srgb_from_array(axis.to_array_f32()));

        #[cfg(not(feature = "f64"))]
        let material = StandardMaterial::from_color(Color::srgb_from_array(axis.to_array()));

        materials.add(material)
    });
    let bundles = tree
        .iter()
        .map(|(bounds, data)| {
            let axis = data.hash.axis();
            (
                meshes.add(mesh_builder.build(bounds, data)),
                materials[axis as usize].clone(),
                data.center.as_vec3(),
            )
        })
        .map(|(mesh_handle, material_handle, translation)| {
            ChunkBundle::new(mesh_handle, material_handle, translation)
        })
        .collect::<Vec<ChunkBundle>>();

    commands.spawn_batch(bundles);
    commands.spawn(PanOrbitCamera {
        radius: Some((RADIUS * 3.0) as f32),
        ..Default::default()
    });
}

fn draw_gizmos(mut gizmos: Gizmos, chunk_query: Query<&GlobalTransform, With<Mesh3d>>) {
    for pos in chunk_query.iter() {
        gizmos.sphere(pos.translation(), 100.0, FUCHSIA);
    }
}
