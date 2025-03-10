use bevy::{
    asset::RenderAssetUsages,
    math::{Dir3, Rect, Vec2, Vec3, Vec3Swizzles},
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use super::{
    cube_tree::Axis,
    helpers::{spherical_uv, unit_cube_to_sphere},
};

#[allow(unused)]
trait RectExtension {
    fn inverse_lerp(&self, point: Vec2) -> Vec2;
    fn contains_rect(&self, rhs: Rect) -> bool;
}

impl RectExtension for Rect {
    #[inline]
    fn inverse_lerp(&self, point: Vec2) -> Vec2 {
        Vec2::new(
            (point.x - self.min.x) / (self.max.x - self.min.x),
            (point.y - self.min.y) / (self.max.y - self.min.y),
        )
    }
    #[inline(always)]
    fn contains_rect(&self, rhs: Rect) -> bool {
        self.contains(rhs.min) && self.contains(rhs.max)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkMeshBuilder {
    radius: f32,
}

#[allow(unused)]
impl ChunkMeshBuilder {
    const SUBDIVISIONS: u32 = 6;
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }

    pub fn build(&self, bounds: Rect, axis: Axis) -> Mesh {
        let vertex_count = Self::SUBDIVISIONS + 2;

        let num_vertices = (vertex_count * 2) as usize;
        let num_indices = ((vertex_count - 1) * (vertex_count - 1) * 6) as usize;

        let mut positions: Vec<Vec3> = Vec::with_capacity(num_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_vertices);
        let mut indices: Vec<u32> = Vec::with_capacity(num_indices);

        let normal = Dir3::from(axis);
        let local_x = normal.yzx();
        let local_y = normal.cross(local_x);
        let size = Vec2::splat(self.radius * 2.0);

        let bounds_min = bounds.min / size;
        let bounds_max = bounds.max / size;

        let step_x = (bounds_max.x - bounds_min.x) / (vertex_count - 1) as f32;
        let step_y = (bounds_max.y - bounds_min.y) / (vertex_count - 1) as f32;

        for y in 0..vertex_count {
            for x in 0..vertex_count {
                let p_x = bounds_min.x + x as f32 * step_x;
                let p_y = bounds_min.y + y as f32 * step_y;

                let pos_on_cube = normal.as_vec3() + p_x * 2.0 * local_x + p_y * 2.0 * local_y;

                let pos = unit_cube_to_sphere(pos_on_cube);
                positions.push(pos * self.radius);
                normals.push(pos.normalize().to_array());
                uvs.push(spherical_uv(pos));

                if x < vertex_count - 1 && y < vertex_count - 1 {
                    let i = x + y * vertex_count;
                    match (p_x < 0.0, p_y < 0.0) {
                        (false, false) => {
                            indices.push(i);
                            indices.push(i + vertex_count + 1);
                            indices.push(i + vertex_count);

                            indices.push(i);
                            indices.push(i + 1);
                            indices.push(i + vertex_count + 1);
                        }
                        (true, false) => {
                            indices.push(i);
                            indices.push(i + 1);
                            indices.push(i + vertex_count);

                            indices.push(i + 1);
                            indices.push(i + vertex_count + 1);
                            indices.push(i + vertex_count);
                        }
                        (false, true) => {
                            indices.push(i);
                            indices.push(i + 1);
                            indices.push(i + vertex_count);

                            indices.push(i + vertex_count);
                            indices.push(i + 1);
                            indices.push(i + vertex_count + 1);
                        }
                        (true, true) => {
                            indices.push(i);
                            indices.push(i + vertex_count + 1);
                            indices.push(i + vertex_count);

                            indices.push(i + 1);
                            indices.push(i + vertex_count + 1);
                            indices.push(i);
                        }
                    }
                }
            }
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}
