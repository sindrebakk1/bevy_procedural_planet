#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use avian3d::math::*;
use bevy::color::palettes::css::{DARK_SEA_GREEN, INDIAN_RED};
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use big_space::camera::{CameraController, CameraControllerPlugin};
use big_space::prelude::*;

use procedural_planet::materials::GlobalMaterialsPlugin;
use procedural_planet::plugins::player::controls::grab_ungrab_mouse;
use procedural_planet::plugins::terrain::cube_tree::{ChunkHash, CubeTree};
use procedural_planet::plugins::terrain::{Body, BodyPreset, GenerateMeshes, Radius};
// use procedural_planet::plugins::terrain::material::TerrainMaterials;
use procedural_planet::plugins::terrain::mesh::ChunkMeshBuilder;
// use procedural_planet::plugins::TerrainPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            WorldInspectorPlugin::default(),
            WireframePlugin,
            BigSpacePlugin::<i64>::default(),
            FloatingOriginDebugPlugin::<i64>::default(),
            GlobalMaterialsPlugin,
            CameraControllerPlugin::<i64>::default(),
        ))
        .init_resource::<ChunkMaterials>()
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
        .add_systems(
            Update,
            (
                toggle_wireframe.run_if(resource_changed::<ButtonInput<KeyCode>>),
                track_target_position,
                grab_ungrab_mouse,
            ),
        )
        .add_observer(generate_meshes::<5>);

    app.run();
}

#[derive(Component, Default)]
struct PlayerCamera;

fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        wireframe_config.global = !wireframe_config.global;
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 120_000.,
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 10_000.0,
            first_cascade_far_bound: 100.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));
    let mut camera_pos = Default::default();
    commands.spawn_big_space_default(|root: &mut GridCommands<i64>| {
        root.insert(Name::new("System"));
        let entity = root
            .with_grid_default(|planet| {
                let body_preset = BodyPreset::MOON;
                camera_pos = Vector::X * body_preset.radius * 1.2;

                planet.insert((Body::from_preset(body_preset), Name::new("Planet")));

                let (camera_cell, camera_translation) =
                    planet.grid().translation_to_grid(camera_pos);

                planet.spawn_spatial((
                    PlayerCamera,
                    Camera3d::default(),
                    Transform::from_translation(camera_translation),
                    camera_cell,
                    FloatingOrigin,
                    CameraController::default(),
                ));
            })
            .id();
        root.commands()
            .entity(entity)
            .trigger(GenerateMeshes(camera_pos));
    });
}

#[allow(clippy::type_complexity)]
fn track_target_position(
    mut commands: Commands,
    target_query: Query<(&GridCell<i64>, &Transform, &Parent), With<PlayerCamera>>,
    mut planet_query: Query<
        (
            Entity,
            &Radius,
            &Grid<i64>,
            &GridCell<i64>,
            &Transform,
            &mut CubeTree,
        ),
        With<Body>,
    >,
    mut prev_position: Local<Vector>,
) {
    let Ok((target_cell, target_transform, parent)) = target_query.get_single() else {
        return;
    };
    let Ok((entity, radius, grid, planet_cell, planet_transform, mut cube_tree)) =
        planet_query.get_mut(parent.get())
    else {
        return;
    };
    let target_position = grid
        .grid_position_double(target_cell, target_transform)
        .adjust_precision();
    let planet_position = grid
        .grid_position_double(planet_cell, planet_transform)
        .adjust_precision();
    let relative_pos = target_position - planet_position;
    if target_position.distance(*prev_position)
        < (target_position.distance(planet_position) - **radius) * 0.01
    {
        return;
    }
    *prev_position = target_position;
    // info!("target_position: {target_position:?}");
    // info!("planet_position: {planet_position:?}");
    // info!("relative_pos: {relative_pos:?}");
    cube_tree.insert(relative_pos);
    commands
        .entity(entity)
        .trigger(GenerateMeshes(relative_pos));
}

#[derive(Bundle)]
struct ChunkBundle(
    Name,
    Mesh3d,
    MeshMaterial3d<StandardMaterial>,
    GridCell<i64>,
    Transform,
);

#[derive(Resource)]
struct ChunkMaterials {
    standard: Handle<StandardMaterial>,
    error: Handle<StandardMaterial>,
}

impl ChunkMaterials {
    pub fn standard(&self) -> Handle<StandardMaterial> {
        self.standard.clone()
    }
    pub fn error(&self) -> Handle<StandardMaterial> {
        self.error.clone()
    }
}

impl FromWorld for ChunkMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self {
            standard: materials.add(StandardMaterial::from_color(DARK_SEA_GREEN)),
            error: materials.add(StandardMaterial::from_color(INDIAN_RED)),
        }
    }
}

#[allow(clippy::type_complexity)]
fn generate_meshes<const SUBDIVISIONS: usize>(
    trigger: Trigger<GenerateMeshes>,
    mut commands: Commands,
    planet_query: Query<(&CubeTree, &Grid<i64>, &GridCell<i64>, &Transform), With<Body>>,
    mut spawned_chunks: Local<Vec<Entity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<ChunkMaterials>,
) where
    [(); (SUBDIVISIONS + 2).pow(2)]:,
    [(); (SUBDIVISIONS + 1).pow(2) * 6]:,
{
    let body_entity = trigger.entity();
    let Ok((cube_tree, grid, grid_cell, transform)) = planet_query.get(body_entity) else {
        return;
    };

    for entity in spawned_chunks.drain(..) {
        commands.entity(entity).remove_parent().despawn();
    }

    let mut hash_set: HashSet<ChunkHash> = HashSet::with_capacity(cube_tree.iter().count());

    let mesh_builder = ChunkMeshBuilder::<SUBDIVISIONS>::new(cube_tree.radius);
    let planet_pos = grid.grid_position_double(grid_cell, transform);
    commands.entity(body_entity).with_children(|parent| {
        for (&bounds, &data) in cube_tree.iter() {
            let (grid_cell, translation) = grid.translation_to_grid(data.center - planet_pos);
            let entity = parent
                .spawn(ChunkBundle(
                    Name::new(format!("{:?}", data.hash.values())),
                    Mesh3d(meshes.add(mesh_builder.build(&bounds, &data))),
                    MeshMaterial3d(if hash_set.contains(&data.hash) {
                        materials.error()
                    } else {
                        materials.standard()
                    }),
                    grid_cell,
                    Transform::from_translation(translation),
                ))
                .id();
            hash_set.insert(data.hash);
            spawned_chunks.push(entity);
        }
    });
}
