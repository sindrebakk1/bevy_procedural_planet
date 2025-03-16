use std::ops::{Index, IndexMut};
use avian3d::math::{Scalar, Vector, Vector2};
use bevy::prelude::*;
use bevy::utils::HashMap;
use lazy_static::lazy_static;

use crate::math::quad_tree::QuadTreeLeafIterMut;
use crate::math::{
    quad_tree::{QuadTreeLeafIter, QuadTreeNode},
    Rectangle,
};
use crate::plugins::terrain::helpers::unit_cube_to_sphere;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
    NegX = 3,
    NegY = 4,
    NegZ = 5,
}

impl Axis {
    pub const ALL: [Self; 6] = [
        Self::X,
        Self::Y,
        Self::Z,
        Self::NegX,
        Self::NegY,
        Self::NegZ,
    ];

    pub fn to_array_generic<T: num_traits::Float>(&self) -> [T; 3] {
        match self {
            Axis::X => [T::one(), T::zero(), T::zero()],
            Axis::Y => [T::zero(), T::one(), T::zero()],
            Axis::Z => [T::zero(), T::zero(), T::one()],
            Axis::NegX => [-T::one(), T::zero(), T::zero()],
            Axis::NegY => [T::zero(), -T::one(), T::zero()],
            Axis::NegZ => [T::zero(), T::zero(), -T::one()],
        }
    }

    #[inline]
    pub fn to_array(&self) -> [Scalar; 3] {
        self.to_array_generic()
    }

    #[cfg(feature = "f64")]
    #[inline]
    pub fn to_array_f32(&self) -> [f32; 3] {
        self.to_array_generic()
    }
}

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

impl From<Axis> for Dir3 {
    fn from(value: Axis) -> Self {
        match value {
            Axis::X => Self::X,
            Axis::Y => Self::Y,
            Axis::Z => Self::Z,
            Axis::NegX => Self::NEG_X,
            Axis::NegY => Self::NEG_Y,
            Axis::NegZ => Self::NEG_Z,
        }
    }
}

impl From<&Axis> for Dir3 {
    fn from(value: &Axis) -> Self {
        match *value {
            Axis::X => Self::X,
            Axis::Y => Self::Y,
            Axis::Z => Self::Z,
            Axis::NegX => Self::NEG_X,
            Axis::NegY => Self::NEG_Y,
            Axis::NegZ => Self::NEG_Z,
        }
    }
}

impl From<Axis> for Vector {
    fn from(value: Axis) -> Self {
        match value {
            Axis::X => Self::X,
            Axis::Y => Self::Y,
            Axis::Z => Self::Z,
            Axis::NegX => Self::NEG_X,
            Axis::NegY => Self::NEG_Y,
            Axis::NegZ => Self::NEG_Z,
        }
    }
}

impl From<&Axis> for Vector {
    fn from(value: &Axis) -> Self {
        match value {
            Axis::X => Self::X,
            Axis::Y => Self::Y,
            Axis::Z => Self::Z,
            Axis::NegX => Self::NEG_X,
            Axis::NegY => Self::NEG_Y,
            Axis::NegZ => Self::NEG_Z,
        }
    }
}

impl std::ops::Mul<Scalar> for Axis {
    type Output = Vector;

    fn mul(self, rhs: Scalar) -> Self::Output {
        Vector::from_array(self.to_array().map(|v| v * rhs))
    }
}

impl std::ops::Mul<Scalar> for &Axis {
    type Output = Vector;

    fn mul(self, rhs: Scalar) -> Self::Output {
        Vector::from_array(self.to_array().map(|v| v * rhs))
    }
}

#[cfg(feature = "f64")]
impl std::ops::Mul<f32> for &Axis {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3::from_array(self.to_array_f32().map(|v| v * rhs))
    }
}

/// TODO: Store Axis in ChunkData, and consider pre-computing absolute center position on insert, also, maybe encode some value that can be used for accurate hashing?
pub type ChunkData = bool;

#[derive(Component, Clone, Debug)]
pub struct CubeTree {
    pub radius: Scalar,
    pub faces: [(Axis, QuadTreeNode<ChunkData>); 6],
}

#[allow(unused)]
impl CubeTree {
    const MIN_SIZE: Scalar = 24.0;
    const THRESHOLD: Scalar = 1.5;
    
    pub fn new(radius: Scalar) -> Self {
        Self {
            radius,
            faces: Axis::ALL.map(|axis| {
                (
                    axis,
                    QuadTreeNode::new(
                        Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(radius)),
                        ChunkData::default(),
                    ),
                )
            }),
        }
    }
    
    pub fn with_data(radius: Scalar, data: ChunkData) -> Self {
        Self {
            radius,
            faces: Axis::ALL.map(|axis| {
                (
                    axis,
                    QuadTreeNode::new(
                        Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(radius)),
                        data,
                    ),
                )
            }),
        }
    }

    pub fn with_subdivisions(radius: Scalar, subdivisions: usize) -> Self {
        Self {
            radius,
            faces: Axis::ALL.map(|axis| {
                (
                    axis,
                    QuadTreeNode::with_subdivisions(
                        Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(radius)),
                        ChunkData::default(),
                        subdivisions,
                    ),
                )
            }),
        }
    }
    
    pub fn insert(&mut self, point: Vector) {
        for axis in Axis::ALL {
            let mut new_node = QuadTreeNode::new(
                Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(self.radius)),
                ChunkData::default(),
            );
            new_node.insert_with(|bounds, data| {
                let size = bounds.size().x;
                if size <= Self::MIN_SIZE {
                    *data = true;
                    return true;
                }
                if self.center_on_sphere(axis, bounds).distance(point) > size * Self::THRESHOLD {
                    return true;
                }
                false
            });
            self[axis] = new_node;
        }
    }
    
    fn center_on_sphere(&self, axis: Axis, bounds: &Rectangle) -> Vector {
        let (axis_normal, local_x, local_y) = AXIS_COORDINATE_FRAMES[&axis];
        let size = Vector2::splat(self.radius * 2.0);

        let bounds_min = bounds.min / size;
        let bounds_max = bounds.max / size;

        let center_pos_on_cube = axis_normal
            + ((bounds_min.x + bounds_max.x) / 2.0) * local_x
            + ((bounds_min.y + bounds_max.y) / 2.0) * local_y;
        
        unit_cube_to_sphere(center_pos_on_cube) * self.radius
    }

    pub fn iter(&self) -> CubeTreeIter {
        CubeTreeIter::new(self)
    }

    pub unsafe fn iter_mut(&mut self) -> CubeTreeIterMut {
        CubeTreeIterMut::new(self)
    }

    pub fn iter_with_capacity<const CAPACITY: usize>(&self) -> CubeTreeIter<CAPACITY> {
        CubeTreeIter::new(self)
    }
    pub unsafe fn iter_mut_with_capacity<const CAPACITY: usize>(&mut self) -> CubeTreeIterMut<CAPACITY> {
        CubeTreeIterMut::new(self)
    }
}

impl Index<Axis> for CubeTree {
    type Output = QuadTreeNode<ChunkData>;

    fn index(&self, index: Axis) -> &Self::Output {
        // Logic to find and return a reference to the element
        // at the specified index
        &self.faces[index as usize].1
    }
}

// Implement IndexMut trait to enable mutable indexing
impl IndexMut<Axis> for CubeTree {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        // Logic to find and return a mutable reference to the element
        // at the specified index
        &mut self.faces[index as usize].1
    }
}

pub struct CubeTreeIter<'a, const CAPACITY: usize = 1024> {
    index: usize,
    faces: &'a [(Axis, QuadTreeNode<ChunkData>); 6],
    chunk_iter: QuadTreeLeafIter<'a, ChunkData, CAPACITY>,
}

impl<'a, const CAPACITY: usize> CubeTreeIter<'a, CAPACITY> {
    pub fn new(cube_tree: &'a CubeTree) -> Self {
        Self {
            index: 0,
            faces: &cube_tree.faces,
            chunk_iter: QuadTreeLeafIter::new(&cube_tree.faces[0].1),
        }
    }
}

impl<'a, const CAPACITY: usize> Iterator for CubeTreeIter<'a, CAPACITY> {
    type Item = (Axis, &'a Rectangle, &'a ChunkData);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((bounds, data)) = self.chunk_iter.next() {
            Some((self.faces[self.index].0, bounds, data))
        } else if self.index < self.faces.len() {
            self.index += 1;
            self.chunk_iter = QuadTreeLeafIter::new(&self.faces[self.index].1);
            self.next()
        } else {
            None
        }
    }
}

pub struct CubeTreeIterMut<'a, const CAPACITY: usize = 1024> {
    index: usize,
    faces: &'a mut [(Axis, QuadTreeNode<ChunkData>)],
    chunk_iter: Option<QuadTreeLeafIterMut<'a, ChunkData, CAPACITY>>,
}

impl<'a, const CAPACITY: usize> CubeTreeIterMut<'a, CAPACITY> {
    pub unsafe fn new(cube_tree: &'a mut CubeTree) -> Self {
        Self {
            index: 0,
            faces: cube_tree.faces.as_mut_slice(),
            chunk_iter: None,
        }
    }
}


/// This implementation is kind of sketchy, use on your own risk
impl<'a, const CAPACITY: usize> Iterator for CubeTreeIterMut<'a, CAPACITY> {
    type Item = (Axis, &'a mut Rectangle, &'a mut ChunkData);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // If we have a current iterator, try to get the next item
            if let Some(iter) = &mut self.chunk_iter {
                if let Some((bounds, data)) = iter.next() {
                    let axis = self.faces[self.index].0;
                    return Some((axis, bounds, data));
                } else {
                    // This iterator is exhausted, move to the next face
                    self.index += 1;
                    self.chunk_iter = None;
                }
            } else {
                if self.index >= self.faces.len() {
                    return None;
                }

                // Create a new iterator for the current face
                // This is tricky because of lifetimes - we need to split the borrow
                let faces_ptr = self.faces.as_mut_ptr();

                // SAFETY: We know self.index is in bounds, and we're only borrowing one element
                unsafe {
                    let face = &mut *faces_ptr.add(self.index);
                    self.chunk_iter = Some(QuadTreeLeafIterMut::new(&mut face.1));
                }
            }
        }
    }
}
