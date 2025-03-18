use avian3d::math::Vector2;
use smallvec::{smallvec, SmallVec};

use super::Rectangle;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Quadrant {
    ROOT = 0,
    SW = 1,
    SE = 2,
    NW = 3,
    NE = 4,
}

impl Quadrant {
    pub const ALL: [Self; 4] = [Quadrant::SW, Quadrant::SE, Quadrant::NW, Quadrant::NE];
}

impl From<u16> for Quadrant {
    fn from(value: u16) -> Self {
        match value {
            0 => Quadrant::ROOT,
            1 => Quadrant::SW,
            2 => Quadrant::SE,
            3 => Quadrant::NW,
            4 => Quadrant::NE,
            _ => panic!("invalid quadrant"),
        }
    }
}

/// A memory-efficient quadtree node that can either be an internal node with four children
/// or a leaf node containing generic data that implements Copy + Clone.
#[derive(Clone)]
pub enum QuadTreeNode<T: Clone> {
    Internal {
        bounds: Rectangle,
        children: [Box<Self>; 4],
    },
    Leaf {
        bounds: Rectangle,
        data: T,
    },
}

impl<T: Clone> QuadTreeNode<T> {
    /// Creates a new leaf node with given bounds and data.
    #[inline]
    pub fn new(bounds: Rectangle, data: T) -> Self {
        Self::Leaf { bounds, data }
    }

    #[inline]
    pub fn new_subdivided(bounds: Rectangle, data: T) -> Self {
        Self::Internal {
            bounds,
            children: Self::subdivide_bounds(&bounds).map(|(_, child_bounds)| {
                Box::new(Self::Leaf {
                    bounds: child_bounds,
                    data: data.clone(),
                })
            }),
        }
    }

    #[inline]
    pub fn with_subdivisions(bounds: Rectangle, data: T, subdivisions: usize) -> Self {
        let mut node = Self::Leaf { bounds, data };
        node.subdivide_recursive(subdivisions);
        node
    }

    pub fn insert<F>(&mut self, predicate: F)
    where
        F: Fn(&Rectangle, &T) -> bool,
    {
        self.insert_impl(&predicate)
    }

    fn insert_impl<F>(&mut self, predicate: &F)
    where
        F: Fn(&Rectangle, &T) -> bool,
    {
        match self {
            QuadTreeNode::Internal { children, .. } => {
                for child in children {
                    child.insert_impl(predicate);
                }
            }
            QuadTreeNode::Leaf { bounds, data } => {
                if predicate(&*bounds, &*data) {
                    return;
                }
                self.subdivide();
                self.insert_impl(predicate);
            }
        }
    }

    pub fn insert_with<P, F>(&mut self, predicate: P, create_data: F)
    where
        P: Fn(&Rectangle, &T) -> bool,
        F: Fn(Quadrant, &Rectangle, &T) -> T,
    {
        self.insert_with_impl(&predicate, &create_data)
    }

    fn insert_with_impl<P, F>(&mut self, predicate: &P, create_data: &F)
    where
        P: Fn(&Rectangle, &T) -> bool,
        F: Fn(Quadrant, &Rectangle, &T) -> T,
    {
        match self {
            QuadTreeNode::Internal { children, .. } => {
                for child in children {
                    child.insert_with_impl(predicate, create_data);
                }
            }
            QuadTreeNode::Leaf { bounds, data } => {
                if predicate(&*bounds, &*data) {
                    return;
                }
                self.subdivided_with(create_data)
                    .insert_with_impl(predicate, create_data);
            }
        }
    }

    /// Returns the bounds of this node.
    #[inline]
    pub fn bounds(&self) -> &Rectangle {
        match self {
            Self::Internal { bounds, .. } => bounds,
            Self::Leaf { bounds, .. } => bounds,
        }
    }

    /// Returns a reference to the data if this is a leaf node, or None if it's an internal node.
    #[inline]
    pub fn data(&self) -> Option<&T> {
        match self {
            Self::Leaf { data, .. } => Some(data),
            _ => None,
        }
    }

    /// Returns a clone of the data if this is a leaf node, or None if it's an internal node.
    #[inline]
    pub fn data_clone(&self) -> Option<T> {
        match self {
            Self::Leaf { data, .. } => Some(data.clone()),
            _ => None,
        }
    }

    /// Returns a mutable reference to the data if this is a leaf node, or None if it's an internal node.
    #[inline]
    pub fn data_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Leaf { data, .. } => Some(data),
            _ => None,
        }
    }

    /// Subdivides a leaf node into an internal node with four child leaf nodes.
    /// Each child will receive a clone of the parent's data.
    pub fn subdivide(&mut self) {
        if let Self::Leaf { bounds, data } = self {
            *self = Self::Internal {
                bounds: *bounds,
                children: Self::subdivide_bounds(bounds).map(|(_, child_bounds)| {
                    Box::new(Self::Leaf {
                        bounds: child_bounds,
                        data: data.clone(),
                    })
                }),
            };
        } else {
            panic!("Cannot subdivide an internal node");
        }
    }

    pub fn subdivided(&mut self) -> &mut Self {
        if let Self::Leaf { bounds, data } = self {
            *self = Self::Internal {
                bounds: *bounds,
                children: Self::subdivide_bounds(bounds).map(|(_, child_bounds)| {
                    Box::new(Self::Leaf {
                        bounds: child_bounds,
                        data: data.clone(),
                    })
                }),
            };
            self
        } else {
            panic!("Cannot subdivide an internal node");
        }
    }

    /// Subdivides a leaf node into an internal node with four child leaf nodes.
    /// The provided function is used to create data for each new child.
    pub fn subdivide_with<F>(&mut self, create_data: F)
    where
        F: Fn(Quadrant, &Rectangle, &T) -> T,
    {
        if let Self::Leaf { bounds, data } = self {
            *self = Self::Internal {
                bounds: *bounds,
                children: Self::subdivide_bounds(bounds).map(|(quadrant, child_bounds)| {
                    Box::new(Self::Leaf {
                        bounds: child_bounds,
                        data: create_data(quadrant, &child_bounds, data),
                    })
                }),
            };
        } else {
            panic!("Cannot subdivide an internal node");
        }
    }

    pub fn subdivided_with<F>(&mut self, create_data: F) -> &mut Self
    where
        F: Fn(Quadrant, &Rectangle, &T) -> T,
    {
        if let Self::Leaf { bounds, data } = self {
            *self = Self::Internal {
                bounds: *bounds,
                children: Self::subdivide_bounds(bounds).map(|(quadrant, child_bounds)| {
                    Box::new(Self::Leaf {
                        bounds: child_bounds,
                        data: create_data(quadrant, &child_bounds, data),
                    })
                }),
            };
            self
        } else {
            panic!("Cannot subdivide an internal node");
        }
    }

    fn subdivide_bounds(bounds: &Rectangle) -> [(Quadrant, Rectangle); 4] {
        let center = bounds.center();
        [
            // Bottom-left
            (Quadrant::SW, Rectangle::from_corners(bounds.min, center)),
            // Bottom-right
            (
                Quadrant::SE,
                Rectangle::from_corners(
                    Vector2::new(center.x, bounds.min.y),
                    Vector2::new(bounds.max.x, center.y),
                ),
            ),
            // Top-left
            (
                Quadrant::NW,
                Rectangle::from_corners(
                    Vector2::new(bounds.min.x, center.y),
                    Vector2::new(center.x, bounds.max.y),
                ),
            ),
            // Top-right
            (Quadrant::NE, Rectangle::from_corners(center, bounds.max)),
        ]
    }

    /// Creates a new node that's subdivided recursively to the specified depth.
    /// Each child will receive a clone of the parent's data.
    #[inline]
    pub fn subdivide_recursive(&mut self, max_depth: usize) {
        self.subdivide_recursive_impl(max_depth, 0)
    }

    fn subdivide_recursive_impl(&mut self, max_depth: usize, current_depth: usize) {
        if current_depth >= max_depth {
            return;
        }

        if let Self::Internal { children, .. } = self.subdivided() {
            for child in children.iter_mut() {
                child.subdivide_recursive_impl(max_depth, current_depth + 1);
            }
        }
    }

    /// Creates a new node that's subdivided recursively to the specified depth.
    /// Each child will receive a clone of the parent's data.
    #[inline]
    pub fn subdivide_recursive_with<F>(&mut self, max_depth: usize, create_data: F)
    where
        F: Fn(Quadrant, usize, &Rectangle, &T) -> T,
    {
        self.subdivide_recursive_with_impl(max_depth, 0, &create_data)
    }

    fn subdivide_recursive_with_impl<F>(
        &mut self,
        max_depth: usize,
        current_depth: usize,
        create_data: &F,
    ) where
        F: Fn(Quadrant, usize, &Rectangle, &T) -> T,
    {
        if current_depth >= max_depth {
            return;
        }

        if let Self::Leaf { bounds, data } = &self {
            *self = Self::Internal {
                bounds: *bounds,
                children: Self::subdivide_bounds(bounds).map(|(quadrant, child_bounds)| {
                    Box::new(Self::Leaf {
                        bounds: child_bounds,
                        data: create_data(quadrant, current_depth, &child_bounds, data),
                    })
                }),
            };
        } else {
            panic!("Cannot subdivide an internal node");
        }

        if let Self::Internal { children, .. } = self {
            for child in children.iter_mut() {
                child.subdivide_recursive_with_impl(max_depth, current_depth + 1, create_data);
            }
        }
    }

    /// Gathers all leaf nodes into the provided vector.
    pub fn gather_leaves<'a>(&'a self, out: &mut Vec<&'a Self>) {
        match self {
            Self::Internal { children, .. } => {
                for child in children {
                    child.gather_leaves(out)
                }
            }
            Self::Leaf { .. } => out.push(self),
        }
    }

    /// Returns an iterator over all leaf nodes in the tree.
    #[inline]
    pub fn iter(&self) -> QuadTreeLeafIter<T> {
        QuadTreeLeafIter::new(self)
    }

    /// Returns an iterator with a custom stack size for performance tuning.
    #[inline]
    pub fn iter_with_capacity<const CAPACITY: usize>(&self) -> QuadTreeLeafIter<T, CAPACITY> {
        QuadTreeLeafIter::new(self)
    }
    /// Returns an iterator over all leaf nodes in the tree.
    #[inline]
    pub fn iter_mut(&mut self) -> QuadTreeLeafIterMut<T> {
        QuadTreeLeafIterMut::new(self)
    }

    /// Returns an iterator with a custom stack size for performance tuning.
    #[inline]
    pub fn iter_mut_with_capacity<const CAPACITY: usize>(
        &mut self,
    ) -> QuadTreeLeafIterMut<T, CAPACITY> {
        QuadTreeLeafIterMut::new(self)
    }
}

impl<T: Clone + PartialEq> PartialEq for QuadTreeNode<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Leaf { bounds, data },
                Self::Leaf {
                    bounds: other_bounds,
                    data: other_data,
                },
            ) => bounds == other_bounds && data == other_data,
            (
                Self::Internal { bounds, children },
                Self::Internal {
                    bounds: other_bounds,
                    children: other_children,
                },
            ) => {
                if bounds != other_bounds {
                    return false;
                }

                for (child, other_child) in children.iter().zip(other_children.iter()) {
                    if child != other_child {
                        return false;
                    }
                }

                true
            }
            _ => false,
        }
    }
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for QuadTreeNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal { bounds, .. } => f
                .debug_struct("QuadTreeNode::Internal")
                .field("bounds", bounds)
                .finish(),
            Self::Leaf { bounds, data } => f
                .debug_struct("QuadTreeNode::Leaf")
                .field("bounds", bounds)
                .field("data", data)
                .finish(),
        }
    }
}

unsafe impl<T: Clone + Send> Send for QuadTreeNode<T> {}
unsafe impl<T: Clone + Sync> Sync for QuadTreeNode<T> {}

/// Iterator for traversing a `QuadTree` and returning references to the bounds and data of each leaf.
///
/// The `CAPACITY` const parameter specifies the maximum number of node references
/// stored on the stack before spilling to the heap.
pub struct QuadTreeLeafIter<'a, T: Clone, const CAPACITY: usize = 1024> {
    stack: SmallVec<[&'a QuadTreeNode<T>; CAPACITY]>,
}

impl<'a, T: Clone, const CAPACITY: usize> QuadTreeLeafIter<'a, T, CAPACITY> {
    #[inline]
    pub fn new(root: &'a QuadTreeNode<T>) -> Self {
        Self {
            stack: smallvec![root],
        }
    }
}

impl<'a, T: Clone, const CAPACITY: usize> Iterator for QuadTreeLeafIter<'a, T, CAPACITY> {
    type Item = (&'a Rectangle, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current_node) = self.stack.pop() {
            match current_node {
                QuadTreeNode::Internal { children, .. } => {
                    for i in (0..4).rev() {
                        self.stack.push(&children[i]);
                    }
                    continue;
                }
                QuadTreeNode::Leaf { bounds, data } => {
                    return Some((bounds, data));
                }
            }
        }
        None
    }
}

/// Iterator for traversing a `QuadTree` and returning mutable references to the bounds and data of each leaf.
///
/// The `CAPACITY` const parameter specifies the maximum number of node references
/// stored on the stack before spilling to the heap.
pub struct QuadTreeLeafIterMut<'a, T: Clone, const CAPACITY: usize = 1024> {
    stack: SmallVec<[&'a mut QuadTreeNode<T>; CAPACITY]>,
}

impl<'a, T: Clone, const CAPACITY: usize> QuadTreeLeafIterMut<'a, T, CAPACITY> {
    #[inline]
    pub fn new(root: &'a mut QuadTreeNode<T>) -> Self {
        Self {
            stack: smallvec![root],
        }
    }
}

impl<'a, T: Clone, const CAPACITY: usize> Iterator for QuadTreeLeafIterMut<'a, T, CAPACITY> {
    type Item = (&'a mut Rectangle, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current_node) = self.stack.pop() {
            match current_node {
                QuadTreeNode::Internal { children, .. } => {
                    self.stack
                        .append(&mut SmallVec::<[&'a mut QuadTreeNode<T>; 4]>::from_iter(
                            children.iter_mut().map(|child| child.as_mut()),
                        ));
                    continue;
                }
                QuadTreeNode::Leaf { bounds, data } => {
                    // Return mutable references to bounds and data
                    return Some((bounds, data));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::math::Vector2;

    #[test]
    fn test_new_leaf() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let data = 42;
        let leaf = QuadTreeNode::new(bounds, data);

        match leaf {
            QuadTreeNode::Leaf {
                bounds: leaf_bounds,
                data: leaf_data,
            } => {
                assert_eq!(leaf_bounds, bounds);
                assert_eq!(leaf_data, data);
            }
            _ => panic!("Expected a leaf node"),
        }
    }

    #[test]
    fn test_bounds() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let leaf = QuadTreeNode::new(bounds, 42);

        assert_eq!(leaf.bounds(), &bounds);

        let mut internal = leaf.clone();
        internal.subdivide();

        assert_eq!(internal.bounds(), &bounds);
    }

    #[test]
    fn test_data_accessors() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut leaf = QuadTreeNode::new(bounds, 42);

        // Test data()
        assert_eq!(leaf.data(), Some(&42));

        // Test data_clone()
        assert_eq!(leaf.data_clone(), Some(42));

        // Test data_mut()
        if let Some(data) = leaf.data_mut() {
            *data = 100;
        }
        assert_eq!(leaf.data(), Some(&100));

        // Test that internal nodes return None for data accessors
        leaf.subdivide();
        assert_eq!(leaf.data(), None);
        assert_eq!(leaf.data_clone(), None);
        assert_eq!(leaf.data_mut(), None);
    }

    #[test]
    fn test_subdivide() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut leaf = QuadTreeNode::new(bounds, 42);

        leaf.subdivide();

        match leaf {
            QuadTreeNode::Internal {
                bounds: internal_bounds,
                children,
            } => {
                assert_eq!(internal_bounds, bounds);
                assert_eq!(children.len(), 4);

                // Check that children are properly positioned
                // Bottom-left child
                assert_eq!(
                    children[0].bounds(),
                    &Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(5.0, 5.0))
                );

                // Bottom-right child
                assert_eq!(
                    children[1].bounds(),
                    &Rectangle::from_corners(Vector2::new(5.0, 0.0), Vector2::new(10.0, 5.0))
                );

                // Top-left child
                assert_eq!(
                    children[2].bounds(),
                    &Rectangle::from_corners(Vector2::new(0.0, 5.0), Vector2::new(5.0, 10.0))
                );

                // Top-right child
                assert_eq!(
                    children[3].bounds(),
                    &Rectangle::from_corners(Vector2::new(5.0, 5.0), Vector2::new(10.0, 10.0))
                );

                // Check that all children have the parent's data
                for child in children.iter() {
                    assert_eq!(child.data(), Some(&42));
                }
            }
            _ => panic!("Expected an internal node after subdivision"),
        }
    }

    #[test]
    #[should_panic(expected = "Cannot subdivide an internal node")]
    fn test_subdivide_internal_panics() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut leaf = QuadTreeNode::new(bounds, 42);

        // First subdivision is fine
        leaf.subdivide();

        // Second subdivision should panic
        leaf.subdivide();
    }

    #[test]
    fn test_subdivide_with() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut leaf = QuadTreeNode::new(bounds, 100);

        // Custom function that sets the data based on the child's position
        leaf.subdivide_with(|_, child_bounds, parent_data| {
            let center = child_bounds.center();
            if center.x < 5.0 && center.y < 5.0 {
                // Bottom-left: parent value
                *parent_data
            } else if center.x >= 5.0 && center.y < 5.0 {
                // Bottom-right: double parent value
                *parent_data * 2
            } else if center.x < 5.0 && center.y >= 5.0 {
                // Top-left: triple parent value
                *parent_data * 3
            } else {
                // Top-right: quadruple parent value
                *parent_data * 4
            }
        });

        match leaf {
            QuadTreeNode::Internal { children, .. } => {
                // Check custom data values
                assert_eq!(children[0].data(), Some(&100)); // Bottom-left: original
                assert_eq!(children[1].data(), Some(&200)); // Bottom-right: double
                assert_eq!(children[2].data(), Some(&300)); // Top-left: triple
                assert_eq!(children[3].data(), Some(&400)); // Top-right: quadruple
            }
            _ => panic!("Expected an internal node after subdivision"),
        }
    }

    #[test]
    fn test_subdivide_recursive() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut leaf = QuadTreeNode::new(bounds, 42);

        // Subdivide to depth 2
        leaf.subdivide_recursive(2);

        // Check that we have the right number of leaves (4^2 = 16)
        let mut leaves = Vec::new();
        leaf.gather_leaves(&mut leaves);
        assert_eq!(leaves.len(), 16);

        // Check that all leaves have the original data
        for leaf in leaves {
            assert_eq!(leaf.data(), Some(&42));
        }
    }

    #[test]
    fn test_subdivide_recursive_with() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut root = QuadTreeNode::new(bounds, 42);

        // Subdivide to depth 2
        root.subdivide_recursive_with(2, |_, _, child_bounds, _| child_bounds.size().x as usize);

        // Check that we have the right number of leaves (4^2 = 16)
        assert_eq!(
            root.iter().count(),
            16,
            "should have correct number of nodes after recursive insert"
        );

        // Check that all leaves have the original data
        assert!(root
            .iter()
            .all(|(bounds, data)| { *data == bounds.size().x as usize }))
    }

    #[test]
    fn test_gather_leaves() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut root = QuadTreeNode::new(bounds, 1);

        // Create a tree with varying depths
        root.subdivide();

        if let QuadTreeNode::Internal { children, .. } = &mut root {
            children[0].subdivide();
            if let QuadTreeNode::Internal {
                children: grandchildren,
                ..
            } = &mut *children[0]
            {
                grandchildren[0].subdivide();
            }
        }

        let mut leaves = Vec::new();
        root.gather_leaves(&mut leaves);

        assert_eq!(leaves.len(), 10);

        // All leaves should have data = 1
        for leaf in leaves {
            assert_eq!(leaf.data(), Some(&1));
        }
    }

    #[test]
    fn test_iter() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut root = QuadTreeNode::new(bounds, "root");

        // Create a simple tree
        root.subdivide();

        // Count the number of leaves using the iterator
        let leaves_count = root.iter().count();
        assert_eq!(leaves_count, 4);

        // Check the data of each leaf
        for (_, data) in root.iter() {
            assert_eq!(*data, "root");
        }

        // Test with custom capacity
        let custom_iter = root.iter_with_capacity::<8>();
        assert_eq!(custom_iter.count(), 4);
    }

    #[test]
    fn test_iter_mut() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut root = QuadTreeNode::new(bounds, "root");

        // Create a simple tree
        root.subdivide();

        for (_, data) in root.iter_mut() {
            *data = "modified"
        }

        // Check the data of each leaf
        for (_, data) in root.iter() {
            assert_eq!(*data, "modified");
        }
    }

    #[test]
    fn test_equality() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let leaf1 = QuadTreeNode::new(bounds, 42);
        let leaf2 = QuadTreeNode::new(bounds, 42);
        let leaf3 = QuadTreeNode::new(bounds, 100);

        // Same bounds and data should be equal
        assert_eq!(leaf1, leaf2);

        // Different data should not be equal
        assert_ne!(leaf1, leaf3);

        // Different types should not be equal
        let mut internal = leaf1.clone();
        internal.subdivide();
        assert_ne!(leaf1, internal);

        // Two identical internal nodes should be equal
        let mut internal2 = leaf2.clone();
        internal2.subdivide();
        assert_eq!(internal, internal2);

        // Modify a child in one internal node
        if let QuadTreeNode::Internal { children, .. } = &mut internal2 {
            if let Some(data) = children[0].data_mut() {
                *data = 100;
            }
        }

        // Now they should not be equal
        assert_ne!(internal, internal2);
    }

    #[test]
    fn test_debug_output() {
        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let leaf = QuadTreeNode::new(bounds, 42);

        let debug_str = format!("{:?}", leaf);
        assert!(debug_str.contains("QuadTreeNode::Leaf"));
        assert!(debug_str.contains("bounds"));
        assert!(debug_str.contains("data"));

        let mut internal = leaf.clone();
        internal.subdivide();

        let debug_str = format!("{:?}", internal);
        assert!(debug_str.contains("QuadTreeNode::Internal"));
        assert!(debug_str.contains("bounds"));
    }

    #[test]
    fn test_multithreaded_access() {
        use std::sync::Arc;
        use std::thread;

        let bounds = Rectangle::from_corners(Vector2::new(0.0, 0.0), Vector2::new(10.0, 10.0));
        let mut root = QuadTreeNode::new(bounds, 1);
        root.subdivide_recursive(3);

        let tree = Arc::new(root);
        let mut handles = vec![];

        // Spawn 4 threads that will read from the tree
        for _ in 0..4 {
            let tree_clone = Arc::clone(&tree);
            let handle = thread::spawn(move || {
                let mut leaf_count = 0;
                for _ in tree_clone.iter() {
                    leaf_count += 1;
                }
                leaf_count
            });
            handles.push(handle);
        }

        // All threads should find the same number of leaves (4^3 = 64)
        for handle in handles {
            let leaf_count = handle.join().unwrap();
            assert_eq!(leaf_count, 64);
        }
    }
}
