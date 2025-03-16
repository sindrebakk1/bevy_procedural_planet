use avian3d::math::Scalar;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use procedural_planet::plugins::terrain::cube_tree::CubeTree;
use procedural_planet::{
    materials::GlobalMaterialsPlugin,
    plugins::{physics::GlobalGravity, terrain::{mesh::ChunkMeshBuilder, cube_tree::Axis}},
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(GlobalMaterialsPlugin)
        .add_plugins(WireframePlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .insert_resource(WireframeConfig {
            global: true,
            default_color: Default::default(),
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
        })
        .insert_resource(GlobalGravity::ZERO)
        .add_systems(Startup, setup);

    app.run();
}

#[derive(Bundle)]
pub struct ChunkBundle {
    mesh_3d: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
    transform: Transform,
}

impl ChunkBundle {
    pub fn new(mesh: Handle<Mesh>, material: Handle<StandardMaterial>, translation: Vec3) -> Self {
        Self {
            mesh_3d: Mesh3d(mesh),
            material: MeshMaterial3d(material),
            transform: Transform::from_translation(translation),
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const RADIUS: Scalar = 10.0;
    let tree = CubeTree::with_subdivisions(RADIUS, 1);
    let mesh_builder = ChunkMeshBuilder::<5>::new(RADIUS);
    let materials = Axis::ALL.map(|axis| {
        #[cfg(feature = "f64")]
        let material =
            StandardMaterial::from_color(Color::srgb_from_array(axis.to_array_f32()));

        #[cfg(not(feature = "f64"))]
        let material = StandardMaterial::from_color(Color::srgb_from_array(axis.to_array()));
        
        materials.add(material)
    });
    let bundles = tree.iter()
        .map(|(axis, bounds, _)| {
            let (mesh, position) = mesh_builder.build(axis, bounds);
            (
                meshes.add(mesh),
                materials[axis as usize].clone(),
                position.as_vec3(),
            )
        })
        .map(|(mesh_handle, material_handle, translation)| {
            ChunkBundle::new(mesh_handle, material_handle, translation)
        })
        .collect::<Vec<ChunkBundle>>();

    commands.spawn_batch(bundles);
    commands.spawn((
        PanOrbitCamera {
            radius: Some(20.0),
            ..Default::default()
        },
        Camera::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, -20.0)).looking_to(Dir3::Z, Dir3::Y),
    ));
}
