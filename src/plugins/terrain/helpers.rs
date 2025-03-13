#![allow(unused)]

use avian3d::math::{Scalar, Vector, Vector2, PI};

use super::cube_tree::Axis;

const fn cos_approx(x: Scalar) -> Scalar {
    let x2 = x * x;
    1.0 - (x2 / 2.0) + (x2 * x2 / 24.0) // Approximate cos(x) using a few terms of Taylor series
}

const fn deg_to_rad(degrees: Scalar) -> Scalar {
    degrees * PI / 180.0
}

pub const fn dot_product_from_angle_deg(degrees: Scalar) -> Scalar {
    cos_approx(deg_to_rad(degrees))
}

pub fn cube_to_sphere(pos: Vector, radius: Scalar) -> Vector {
    let p = pos / radius;
    let (x2, y2, z2) = (p.x * p.x, p.y * p.y, p.z * p.z);

    Vector::new(
        p.x * (1.0 - (y2 + z2) / 2.0 + (y2 * z2) / 3.0).sqrt(),
        p.y * (1.0 - (z2 + x2) / 2.0 + (z2 * x2) / 3.0).sqrt(),
        p.z * (1.0 - (x2 + y2) / 2.0 + (x2 * y2) / 3.0).sqrt(),
    ) * radius
}

pub fn unit_cube_to_sphere(pos: Vector) -> Vector {
    let (x2, y2, z2) = (pos.x * pos.x, pos.y * pos.y, pos.z * pos.z);

    Vector::new(
        pos.x * (1.0 - y2 / 2.0 - z2 / 2.0 + y2 * z2 / 3.0).sqrt(),
        pos.y * (1.0 - z2 / 2.0 - x2 / 2.0 + z2 * x2 / 3.0).sqrt(),
        pos.z * (1.0 - x2 / 2.0 - y2 / 2.0 + x2 * y2 / 3.0).sqrt(),
    )
}

pub fn unit_sphere_to_cube(pos: Vector, iterations: usize) -> Vector {
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

    Vector::new(cube_x, cube_y, cube_z)
}

#[inline(always)]
fn sphere_circumference_distance(pos: Vector, target: Vector, r: Scalar) -> Scalar {
    pos.dot(target).acos() * r
}

pub fn spherical_uv(pos: Vector) -> Vector2 {
    let phi = pos.z.atan2(pos.x); // Azimuth
    let theta = (pos.y).acos(); // Elevation
    let u = phi / (2.0 * PI) + 0.5;
    let v = theta / PI;
    Vector2::new(u, v)
}

pub fn get_axis_element(v: Vector, face: Axis) -> Scalar {
    match face {
        Axis::X => v.x,
        Axis::Y => v.y,
        Axis::Z => v.z,
        Axis::NegX => -v.x,
        Axis::NegY => -v.y,
        Axis::NegZ => -v.z,
    }
}
