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

impl From<u32> for Axis {
    fn from(value: u32) -> Self {
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
pub struct ChunkHash(u32);

impl ChunkHash {
    pub fn new(axis: Axis, depth: u8, collider: bool, path: [Quadrant; 7]) -> Self {
        debug_assert!(depth <= 63, "depth is too large for 6 bits");
        let mut hash = (axis as u32) & 0b111;
        hash |= (depth as u32 & 0b111_111) << 3;
        hash |= (collider as u32) << 9;
        for (i, &quadrant) in path.iter().enumerate() {
            let shift = 10 + (i * 3);
            hash |= (quadrant as u32 & 0b111) << shift;
        }
        Self(hash)
    }
    #[inline]
    pub fn new_root(axis: Axis) -> Self {
        Self::new(axis, 0, false, [Quadrant::ROOT; 7])
    }

    pub fn push_quadrant(&self, new_quadrant: Quadrant) -> Self {
        let header_bits = self.0 & 0x3FF; // 0x3FF = 0b11_1111_1111 (first 10 bits)
        let path_bits = (self.0 >> 10) & 0x7FFFFF; // 0x7FFFFF = 23 bits for 7 quadrants minus last one
        let shifted_path = path_bits << 3;
        let new_path = shifted_path | (new_quadrant as u32 & 0b111);
        let result = header_bits | (new_path << 10);

        Self(result)
    }

    #[inline]
    pub fn with_depth(&self, depth: u8) -> Self {
        debug_assert!(depth <= 63, "depth is too large for 6 bits");
        Self((self.0 & !(0b111_111 << 3)) | ((depth as u32 & 0b111_111) << 3))
    }

    #[inline]
    pub fn increment_depth(&self) -> Self {
        self.with_depth(std::cmp::min(self.depth() + 1, 63))
    }

    #[inline]
    pub fn with_collider(&self, collider: bool) -> Self {
        Self((self.0 & !(0b1 << 9)) | ((collider as u32) << 9))
    }

    #[inline]
    pub fn axis(&self) -> Axis {
        Axis::from(self.0 & 0b111)
    }

    #[inline]
    pub fn depth(&self) -> u8 {
        ((self.0 >> 3) & 0b111_111) as u8
    }

    #[inline]
    pub fn collider(&self) -> bool {
        ((self.0 >> 9) & 0b1) != 0
    }

    #[inline]
    pub fn path(&self) -> [Quadrant; 7] {
        let mut path = [Quadrant::ROOT; 7];
        for i in 0..7 {
            let shift = 10 + (i * 3);
            path[i] = Quadrant::from((self.0 >> shift) & 0b111);
        }
        path
    }

    #[inline]
    pub fn values(&self) -> (Axis, u8, [Quadrant; 7], bool) {
        (self.axis(), self.depth(), self.path(), self.collider())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ChunkData {
    pub center: Vector,
    pub hash: ChunkHash,
}

impl ChunkData {
    pub fn new(axis: Axis, bounds: &Rectangle, radius: Scalar, hash: ChunkHash) -> Self {
        Self {
            center: center_on_sphere(axis, radius, bounds),
            hash,
        }
    }

    pub fn new_root(axis: Axis, bounds: &Rectangle, radius: Scalar) -> Self {
        Self {
            center: center_on_sphere(axis, radius, bounds),
            hash: ChunkHash::new_root(axis),
        }
    }
}

pub type CubeTreeNode = QuadTreeNode<ChunkData>;

#[derive(Component, Clone, Debug)]
pub struct CubeTree {
    pub radius: Scalar,
    pub faces: [CubeTreeNode; 6],
}

#[allow(unused)]
impl CubeTree {
    const MIN_SIZE: Scalar = 24.0;
    const THRESHOLD: Scalar = 1.5;
    const COLLIDER_RADIUS: Scalar = 24.0;

    pub fn new(radius: Scalar) -> Self {
        let bounds = Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(radius));
        Self {
            radius,
            faces: Axis::ALL.map(|axis| {
                let hash = ChunkHash::new_root(axis);
                CubeTreeNode::new_subdivided(bounds, |(quadrant, bounds)| {
                    ChunkData::new(
                        axis,
                        bounds,
                        radius,
                        hash.with_depth(1).push_quadrant(quadrant),
                    )
                })
            }),
        }
    }

    pub fn insert(&mut self, point: Vector) {
        let bounds = Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(self.radius));
        for axis in Axis::ALL {
            let hash = ChunkHash::new_root(axis);
            let mut new_node = CubeTreeNode::new_subdivided(bounds, |(quadrant, bounds)| {
                ChunkData::new(
                    axis,
                    bounds,
                    self.radius,
                    hash.with_depth(1).push_quadrant(quadrant),
                )
            });
            new_node.insert_mut(
                |(bounds, data)| {
                    let size = bounds.size().x;
                    if size <= Self::MIN_SIZE {
                        data.hash = data.hash.with_collider(true);
                        return true;
                    }
                    if data.center.distance(point) > size * Self::THRESHOLD {
                        return true;
                    }
                    false
                },
                |(quadrant, bounds, data)| {
                    ChunkData::new(
                        axis,
                        bounds,
                        self.radius,
                        data.hash.increment_depth().push_quadrant(quadrant),
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
        &self.faces[index as usize]
    }
}

// Implement IndexMut trait to enable mutable indexing
impl IndexMut<Axis> for CubeTree {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_chunk_hash() {
        // Setup
        let axis = Axis::X;
        let depth = 5;
        let collider = false;
        let path = [
            Quadrant::NW,
            Quadrant::NE,
            Quadrant::SW,
            Quadrant::SE,
            Quadrant::NW,
            Quadrant::NE,
            Quadrant::SW,
        ];

        // Create a new ChunkHash
        let hash = ChunkHash::new(axis, depth, collider, path);

        // Verify by extracting fields
        assert_eq!(hash.axis(), axis);
        assert_eq!(hash.depth(), depth);
        assert_eq!(hash.collider(), collider);
        assert_eq!(hash.path(), path);
    }

    #[test]
    fn test_new_root() {
        let axis = Axis::Y;
        let hash = ChunkHash::new_root(axis);

        assert_eq!(hash.axis(), axis);
        assert_eq!(hash.depth(), 0);
        assert_eq!(hash.collider(), false);
        assert_eq!(hash.path(), [Quadrant::ROOT; 7]);
    }

    #[test]
    fn test_push_quadrant() {
        // Setup - create an initial hash
        let axis = Axis::Z;
        let depth = 3;
        let collider = true;
        let path = [
            Quadrant::NW,
            Quadrant::NE,
            Quadrant::SW,
            Quadrant::SE,
            Quadrant::NW,
            Quadrant::NE,
            Quadrant::SW,
        ];

        let initial_hash = ChunkHash::new(axis, depth, collider, path);

        // Push a new quadrant to the front
        let new_quadrant = Quadrant::SE;
        let updated_hash = initial_hash.push_quadrant(new_quadrant);

        // Expected new path: the new quadrant at the front, and the last one dropped
        let expected_path = [
            Quadrant::SE,
            Quadrant::NW,
            Quadrant::NE,
            Quadrant::SW,
            Quadrant::SE,
            Quadrant::NW,
            Quadrant::NE,
        ];

        // Verify the path was updated correctly
        assert_eq!(updated_hash.path(), expected_path);

        // Verify other fields remained unchanged
        assert_eq!(updated_hash.axis(), axis);
        assert_eq!(updated_hash.depth(), depth);
        assert_eq!(updated_hash.collider(), collider);
    }

    #[test]
    fn test_with_depth() {
        let initial_hash = ChunkHash::new_root(Axis::X);
        let new_depth = 42;
        let updated_hash = initial_hash.with_depth(new_depth);

        assert_eq!(updated_hash.depth(), new_depth);
        assert_eq!(updated_hash.axis(), initial_hash.axis());
        assert_eq!(updated_hash.collider(), initial_hash.collider());
        assert_eq!(updated_hash.path(), initial_hash.path());
    }

    #[test]
    fn test_with_collider() {
        let initial_hash = ChunkHash::new_root(Axis::Y);
        let updated_hash = initial_hash.with_collider(true);

        assert_eq!(updated_hash.collider(), true);
        assert_eq!(updated_hash.axis(), initial_hash.axis());
        assert_eq!(updated_hash.depth(), initial_hash.depth());
        assert_eq!(updated_hash.path(), initial_hash.path());
    }

    #[test]
    fn test_values() {
        let axis = Axis::Z;
        let depth = 7;
        let collider = true;
        let path = [Quadrant::ROOT; 7];

        let hash = ChunkHash::new(axis, depth, collider, path);
        let (extracted_axis, extracted_depth, extracted_path, extracted_flag) = hash.values();

        assert_eq!(extracted_axis, axis);
        assert_eq!(extracted_depth, depth);
        assert_eq!(extracted_path, path);
        // Note: The method signature shows values() returning flag, but the implementation calls self.flag()
        // which doesn't exist. I'll assume it should call self.collider() instead.
        assert_eq!(extracted_flag, collider);
    }

    #[test]
    #[should_panic(expected = "depth is too large for 6 bits")]
    fn test_depth_too_large() {
        // This should panic because depth > 63
        ChunkHash::new(Axis::X, 64, false, [Quadrant::ROOT; 7]);
    }
}
