#![allow(unused)]
use crate::plugins::terrain::cube_tree::Axis;
use bevy::math::Vec3;

pub fn cube_to_sphere(pos: Vec3, radius: f32) -> Vec3 {
    let p = pos / radius;
    let (x2, y2, z2) = (p.x * p.x, p.y * p.y, p.z * p.z);

    Vec3::new(
        p.x * (1.0 - (y2 + z2) / 2.0 + (y2 * z2) / 3.0).sqrt(),
        p.y * (1.0 - (z2 + x2) / 2.0 + (z2 * x2) / 3.0).sqrt(),
        p.z * (1.0 - (x2 + y2) / 2.0 + (x2 * y2) / 3.0).sqrt(),
    ) * radius
}

pub fn unit_cube_to_sphere(pos: Vec3) -> Vec3 {
    let (x2, y2, z2) = (pos.x * pos.x, pos.y * pos.y, pos.z * pos.z);

    Vec3::new(
        pos.x * (1.0 - y2 / 2.0 - z2 / 2.0 + y2 * z2 / 3.0).sqrt(),
        pos.y * (1.0 - z2 / 2.0 - x2 / 2.0 + z2 * x2 / 3.0).sqrt(),
        pos.z * (1.0 - x2 / 2.0 - y2 / 2.0 + x2 * y2 / 3.0).sqrt(),
    )
}

pub fn unit_sphere_to_cube(pos: Vec3, iterations: usize) -> Vec3 {
    let x = pos.x;
    let y = pos.y;
    let z = pos.z;

    let length = (x * x + y * y + z * z).sqrt();
    let x_normalized = x / length;
    let y_normalized = y / length;
    let z_normalized = z / length;

    let mut cube_x = x_normalized;
    let mut cube_y = y_normalized;
    let mut cube_z = z_normalized;

    for _ in 0..iterations {
        let x2 = cube_x * cube_x;
        let y2 = cube_y * cube_y;
        let z2 = cube_z * cube_z;

        cube_x = x_normalized / (1.0 - y2 / 2.0 - z2 / 2.0 + y2 * z2 / 3.0).sqrt();
        cube_y = y_normalized / (1.0 - z2 / 2.0 - x2 / 2.0 + z2 * x2 / 3.0).sqrt();
        cube_z = z_normalized / (1.0 - x2 / 2.0 - y2 / 2.0 + x2 * y2 / 3.0).sqrt();
    }

    Vec3::new(cube_x, cube_y, cube_z)
}

#[inline(always)]
fn sphere_circumference_distance(pos: Vec3, target: Vec3, r: f32) -> f32 {
    pos.dot(target).acos() * r
}

pub fn spherical_uv(pos: Vec3) -> [f32; 2] {
    let phi = pos.z.atan2(pos.x); // Azimuth
    let theta = (pos.y).acos(); // Elevation
    let u = phi / (2.0 * std::f32::consts::PI) + 0.5;
    let v = theta / std::f32::consts::PI;
    [u, v]
}

pub fn get_axis_element(v: Vec3, face: Axis) -> f32 {
    match face {
        Axis::X => v.x,
        Axis::Y => v.y,
        Axis::Z => v.z,
        Axis::NegX => -v.x,
        Axis::NegY => -v.y,
        Axis::NegZ => -v.z,
    }
}
