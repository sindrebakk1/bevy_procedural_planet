#![allow(unused)]

use super::cube_tree::Axis;
use crate::math::Rectangle;
use avian3d::math::{Scalar, Vector, Vector2, PI};
use avian3d::parry::na::SimdComplexField;
use bevy::math::Vec3Swizzles;
use bevy::utils::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref AXIS_COORDINATE_FRAMES: HashMap<Axis, (Vector, Vector, Vector)> = {
        let mut m = HashMap::new();

        for axis in Axis::ALL {
            let axis_normal = Vector::from(axis);
            let local_y = axis_normal.yzx();
            let local_x = axis_normal.cross(local_y);

            m.insert(axis, (axis_normal, local_x, local_y));
        }

        m
    };
}

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

#[inline]
pub fn cube_to_sphere(pos: Vector, radius: Scalar) -> Vector {
    unit_cube_to_sphere(pos / radius) * radius
}

pub fn unit_cube_to_sphere(pos: Vector) -> Vector {
    let [x, y, z] = pos.to_array();
    let (x2, y2, z2) = (x.simd_powi(2), y.simd_powi(2), z.simd_powi(2));

    Vector::new(
        x * (1.0 - y2 / 2.0 - z2 / 2.0 + y2 * z2 / 3.0).simd_sqrt(),
        y * (1.0 - z2 / 2.0 - x2 / 2.0 + z2 * x2 / 3.0).simd_sqrt(),
        z * (1.0 - x2 / 2.0 - y2 / 2.0 + x2 * y2 / 3.0).simd_sqrt(),
    )
}

pub fn unit_sphere_to_cube(pos: Vector, iterations: usize) -> Vector {
    let [x, y, z] = pos.to_array();

    let length = (x.simd_powi(2) + y.simd_powi(2) + z.simd_powi(2)).simd_sqrt();
    let (x_normalized, y_normalized, z_normalized) = (x / length, y / length, z / length);
    let (mut cube_x, mut cube_y, mut cube_z) = (x_normalized, y_normalized, z_normalized);

    for _ in 0..iterations {
        let (x2, y2, z2) = (
            cube_x.simd_powi(2),
            cube_y.simd_powi(2),
            cube_z.simd_powi(2),
        );

        cube_x = x_normalized / (1.0 - y2 / 2.0 - z2 / 2.0 + y2 * z2 / 3.0).simd_sqrt();
        cube_y = y_normalized / (1.0 - z2 / 2.0 - x2 / 2.0 + z2 * x2 / 3.0).simd_sqrt();
        cube_z = z_normalized / (1.0 - x2 / 2.0 - y2 / 2.0 + x2 * y2 / 3.0).simd_sqrt();
    }

    Vector::new(cube_x, cube_y, cube_z)
}

pub fn spherical_uv(normal: Vector) -> Vector2 {
    debug_assert!(normal.is_normalized(), "normal vector passed to spherical_uv must be normalized");
    let phi = normal.z.atan2(normal.x); // Azimuth
    let theta = (normal.y).acos(); // Elevation
    let u = phi / (2.0 * PI) + 0.5;
    let v = theta / PI;
    Vector2::new(u, v)
}

pub fn center_on_sphere(axis: Axis, radius: Scalar, bounds: &Rectangle) -> Vector {
    let (axis_normal, local_x, local_y) = AXIS_COORDINATE_FRAMES[&axis];
    unit_cube_to_sphere(
        axis_normal
            + local_x * (2.0 * ((bounds.min.x + bounds.max.x) / (radius * 4.0)))
            + local_y * (2.0 * ((bounds.min.y + bounds.max.y) / (radius * 4.0))),
    ) * radius
}
