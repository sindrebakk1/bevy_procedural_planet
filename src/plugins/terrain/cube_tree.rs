use avian3d::math::{Scalar, Vector, Vector2};
use bevy::prelude::*;
use std::ops::{Index, IndexMut};

use crate::math::quad_tree::{QuadTreeLeafIterMut, Quadrant};
use crate::math::{
    quad_tree::{QuadTreeLeafIter, QuadTreeNode},
    Rectangle,
};
use crate::plugins::terrain::helpers::center_on_sphere;

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

impl From<u16> for Axis {
    fn from(value: u16) -> Self {
        match value {
            0 => Axis::X,
            1 => Axis::Y,
            2 => Axis::Z,
            3 => Axis::NegX,
            4 => Axis::NegY,
            5 => Axis::NegZ,
            _ => panic!("invalid face"),
        }
    }
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
impl std::ops::Mul<f32> for Axis {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3::from_array(self.to_array_f32().map(|v| v * rhs))
    }
}

#[cfg(feature = "f64")]
impl std::ops::Mul<f32> for &Axis {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3::from_array(self.to_array_f32().map(|v| v * rhs))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChunkHash(u16);

impl ChunkHash {
    #[inline]
    pub fn new(axis: Axis, quadrant: Quadrant, collider: bool, depth: u8) -> Self {
        let axis_bits = (axis as u16) & 0b111;
        let quadrant_bits = (quadrant as u16 & 0b111) << 3;
        let collider_bit = (collider as u16) << 6;
        let depth_bits = (depth as u16) << 7;

        Self(axis_bits | quadrant_bits | collider_bit | depth_bits)
    }

    #[inline]
    pub fn axis(&self) -> Axis {
        Axis::from(self.0 & 0b111)
    }

    #[inline]
    pub fn quadrant(&self) -> Quadrant {
        Quadrant::from((self.0 >> 3) & 0b111)
    }

    #[inline]
    pub fn collider(&self) -> bool {
        (self.0 >> 6) & 0b1 != 0
    }

    #[inline]
    pub fn depth(&self) -> u8 {
        ((self.0 >> 7) & 0b1111_1111) as u8
    }

    #[inline]
    pub fn values(&self) -> (Axis, Quadrant, bool, u8) {
        (self.axis(), self.quadrant(), self.collider(), self.depth())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ChunkData {
    pub center: Vector,
    pub hash: ChunkHash,
}

impl ChunkData {
    pub fn new(
        axis: Axis,
        quadrant: Quadrant,
        collider: bool,
        depth: u8,
        radius: Scalar,
        bounds: &Rectangle,
    ) -> Self {
        Self {
            center: center_on_sphere(axis, radius, bounds),
            hash: ChunkHash::new(axis, quadrant, collider, depth),
        }
    }
}

pub type CubeTreeNode = QuadTreeNode<ChunkData>;

#[derive(Component, Clone, Debug)]
pub struct CubeTree {
    subdivisions: usize,
    pub radius: Scalar,
    pub faces: [CubeTreeNode; 6],
}

#[allow(unused)]
impl CubeTree {
    const MIN_SIZE: Scalar = 24.0;
    const THRESHOLD: Scalar = 1.5;

    pub fn new(radius: Scalar) -> Self {
        let bounds = Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(radius));
        Self {
            subdivisions: 0,
            radius,
            faces: Axis::ALL.map(|axis| {
                CubeTreeNode::new(
                    bounds,
                    ChunkData::new(axis, Quadrant::ROOT, false, 0, radius, &bounds),
                )
            }),
        }
    }

    pub fn with_subdivisions(radius: Scalar, subdivisions: usize) -> Self {
        let bounds = Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(radius));
        Self {
            subdivisions,
            radius,
            faces: Axis::ALL.map(|axis| {
                let mut root = CubeTreeNode::new(
                    bounds,
                    ChunkData::new(axis, Quadrant::ROOT, false, 0, radius, &bounds),
                );
                root.subdivide_recursive_with(subdivisions, |quadrant, depth, bounds, data| {
                    ChunkData::new(axis, quadrant, false, depth as u8, radius, bounds)
                });
                root
            }),
        }
    }

    pub fn insert(&mut self, point: Vector) {
        let bounds = Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(self.radius));
        for axis in Axis::ALL {
            let mut new_node = CubeTreeNode::new(
                bounds,
                ChunkData::new(axis, Quadrant::ROOT, false, 0, self.radius, &bounds),
            );
            new_node.subdivide_recursive_with(
                self.subdivisions,
                |quadrant, depth, bounds, data| {
                    ChunkData::new(axis, quadrant, false, depth as u8, self.radius, bounds)
                },
            );
            new_node.insert_with(
                |bounds, data| {
                    let size = bounds.size().x;
                    if size <= Self::MIN_SIZE
                        || data.center.distance(point) > size * Self::THRESHOLD
                    {
                        return true;
                    }
                    false
                },
                |quadrant, bounds, data| {
                    ChunkData::new(
                        axis,
                        quadrant,
                        false,
                        data.hash.depth() + 1,
                        self.radius,
                        bounds,
                    )
                },
            );
            self[axis] = new_node;
        }
    }

    pub fn iter(&self) -> CubeTreeIter {
        CubeTreeIter::new(self)
    }

    pub fn iter_with_capacity<const CAPACITY: usize>(&self) -> CubeTreeIter<CAPACITY> {
        CubeTreeIter::new(self)
    }

    /// Returns a mutable iterator over the tree.
    ///
    /// # Safety
    /// This function is marked `unsafe` because the iterator has not been
    /// thoroughly tested for all possible use cases. The caller must ensure that:
    /// - The iterator does not cause data races or aliasing violations.
    /// - The tree structure remains valid while iterating.
    /// - There are no concurrent modifications that could lead to undefined behavior.
    ///
    /// If unsure, use a safe alternative or thoroughly test before usage.
    pub unsafe fn iter_mut(&mut self) -> CubeTreeIterMut {
        CubeTreeIterMut::new(self)
    }

    /// Returns a mutable iterator over the tree with a predefined capacity.
    ///
    /// # Safety
    /// This function is marked `unsafe` for the same reasons as `iter_mut()`.
    /// The caller must ensure that:
    /// - The specified `CAPACITY` is appropriate for safe iteration.
    /// - There are no modifications to the tree that could lead to invalid memory access.
    /// - Proper testing has been conducted to validate correctness in the intended use case.
    pub unsafe fn iter_mut_with_capacity<const CAPACITY: usize>(
        &mut self,
    ) -> CubeTreeIterMut<CAPACITY> {
        CubeTreeIterMut::new(self)
    }
}

impl Index<Axis> for CubeTree {
    type Output = CubeTreeNode;

    fn index(&self, index: Axis) -> &Self::Output {
        // Logic to find and return a reference to the element
        // at the specified index
        &self.faces[index as usize]
    }
}

// Implement IndexMut trait to enable mutable indexing
impl IndexMut<Axis> for CubeTree {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        // Logic to find and return a mutable reference to the element
        // at the specified index
        &mut self.faces[index as usize]
    }
}

pub struct CubeTreeIter<'a, const CAPACITY: usize = 512> {
    index: usize,
    faces: &'a [CubeTreeNode; 6],
    chunk_iter: QuadTreeLeafIter<'a, ChunkData, CAPACITY>,
}

impl<'a, const CAPACITY: usize> CubeTreeIter<'a, CAPACITY> {
    pub fn new(cube_tree: &'a CubeTree) -> Self {
        Self {
            index: 0,
            faces: &cube_tree.faces,
            chunk_iter: QuadTreeLeafIter::new(&cube_tree.faces[0]),
        }
    }
}

impl<'a, const CAPACITY: usize> Iterator for CubeTreeIter<'a, CAPACITY> {
    type Item = (&'a Rectangle, &'a ChunkData);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((bounds, data)) = self.chunk_iter.next() {
            Some((bounds, data))
        } else if self.index < self.faces.len() - 1 {
            self.index += 1;
            self.chunk_iter = QuadTreeLeafIter::new(&self.faces[self.index]);
            self.next()
        } else {
            None
        }
    }
}

pub struct CubeTreeIterMut<'a, const CAPACITY: usize = 512> {
    index: usize,
    faces: &'a mut [CubeTreeNode],
    chunk_iter: Option<QuadTreeLeafIterMut<'a, ChunkData, CAPACITY>>,
}

impl<'a, const CAPACITY: usize> CubeTreeIterMut<'a, CAPACITY> {
    /// Creates a new mutable iterator over a `CubeTree`.
    ///
    /// # Safety
    /// - The caller must ensure that `cube_tree` remains valid for the duration of the iterator.
    /// - There must be no other mutable references to `cube_tree.faces` while this iterator exists.
    /// - The iterator must not be used in a way that causes data races or aliasing violations.
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
            if self.index >= self.faces.len() {
                return None;
            }
            // If we have a current iterator, try to get the next item
            if let Some(iter) = &mut self.chunk_iter {
                if let Some((bounds, data)) = iter.next() {
                    return Some((data.hash.axis(), bounds, data));
                } else {
                    // This iterator is exhausted, move to the next face
                    self.index += 1;
                    self.chunk_iter = None;
                }
            } else {
                // Create a new iterator for the current face
                // This is tricky because of lifetimes - we need to split the borrow
                let faces_ptr = self.faces.as_mut_ptr();

                // SAFETY: We know self.index is in bounds, and we're only borrowing one element
                unsafe {
                    let face = &mut *faces_ptr.add(self.index);
                    self.chunk_iter = Some(QuadTreeLeafIterMut::new(face));
                }
            }
        }
    }
}
