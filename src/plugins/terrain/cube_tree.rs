use avian3d::math::{Scalar, Vector, Vector2};
use bevy::prelude::*;

use super::helpers::cube_to_sphere;
use crate::math::Rectangle;

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

#[derive(Component, Clone, Debug)]
pub struct CubeTree {
    pub faces: [(Axis, CubeTreeNode); 6],
    half_size: Scalar,
}

#[allow(unused)]
impl CubeTree {
    pub fn new(half_size: Scalar) -> Self {
        let faces = Axis::ALL.map(|axis| (axis, CubeTreeNode::new(half_size, axis)));
        Self { faces, half_size }
    }

    #[inline]
    pub fn get(&self, axis: Axis) -> &CubeTreeNode {
        &self.faces[axis as usize].1
    }

    #[inline]
    pub fn get_mut(&mut self, axis: Axis) -> &mut CubeTreeNode {
        &mut self.faces[axis as usize].1
    }

    pub fn set_root_node(&mut self, axis: Axis, node: CubeTreeNode) {
        self.faces[axis as usize] = (axis, node)
    }

    pub fn insert(&mut self, relative_pos: Vector) {
        for axis in Axis::ALL {
            let mut new_node = CubeTreeNode::new(self.half_size, axis);
            new_node.insert(relative_pos);
            self.set_root_node(axis, new_node);
        }
    }
}

#[derive(Clone)]
pub enum CubeTreeNode {
    Internal {
        bounds: Rectangle,
        children: [Box<Self>; 4],
    },
    Leaf {
        collider: bool,
        half_size: Scalar,
        face: Axis,
        bounds: Rectangle,
    },
}

impl CubeTreeNode {
    const MIN_SIZE: Scalar = 24.0;
    const THRESHOLD: Scalar = 1.5;

    pub fn new(half_size: Scalar, face: Axis) -> Self {
        let mut node = Self::Leaf {
            collider: false,
            half_size,
            face,
            bounds: Rectangle::from_center_half_size(Vector2::ZERO, Vector2::splat(half_size)),
        };
        node.subdivide();
        node
    }

    pub fn bounds(&self) -> Rectangle {
        match *self {
            CubeTreeNode::Internal { bounds, .. } => bounds,
            CubeTreeNode::Leaf { bounds, .. } => bounds,
        }
    }

    pub fn insert(&mut self, point: Vector) {
        match self {
            CubeTreeNode::Internal {
                ref mut children, ..
            } => {
                for child in children {
                    child.insert(point);
                }
            }
            CubeTreeNode::Leaf { bounds, .. } => {
                let size = bounds.size().x;
                if size <= Self::MIN_SIZE {
                    self.set_collider(true);
                    return;
                }
                if self.center().unwrap().distance(point) > size * Self::THRESHOLD {
                    return;
                }
                self.subdivide();
                self.insert(point);
            }
        }
    }

    #[inline]
    pub fn collider(&self) -> bool {
        match *self {
            CubeTreeNode::Leaf { collider, .. } => collider,
            _ => false,
        }
    }

    #[inline]
    pub fn set_collider(&mut self, value: bool) {
        if let CubeTreeNode::Leaf { collider, .. } = self {
            *collider = value
        };
    }

    pub fn center(&self) -> Option<Vector> {
        match *self {
            CubeTreeNode::Leaf {
                half_size,
                face,
                bounds,
                ..
            } => {
                let [x, y] = bounds.center().to_array();
                let point_on_cube = match face {
                    Axis::X => Vector::new(half_size, -y, x),
                    Axis::Y => Vector::new(x, half_size, -y),
                    Axis::Z => Vector::new(-y, x, half_size),
                    Axis::NegX => Vector::new(-half_size, -y, -x),
                    Axis::NegY => Vector::new(-x, -half_size, -y),
                    Axis::NegZ => Vector::new(-y, -x, -half_size),
                };
                Some(cube_to_sphere(point_on_cube, half_size))
            }
            _ => None,
        }
    }

    #[inline]
    pub fn normal(&self) -> Option<Vector> {
        self.center().map(|center| center.normalize())
    }

    pub fn gather_children(&self, out: &mut Vec<Self>) {
        match self {
            Self::Internal { children, .. } => {
                for child in children {
                    child.gather_children(out)
                }
            }
            Self::Leaf { .. } => out.push(self.clone()),
        }
    }

    pub fn gather_filtered_children(
        &self,
        out: &mut Vec<Self>,
        mut predicate: impl FnMut(&Self) -> bool,
    ) {
        match self {
            Self::Internal { children, .. } => {
                for child in children {
                    child.gather_children(out)
                }
            }
            Self::Leaf { .. } => {
                if predicate(self) {
                    out.push(self.clone());
                }
            }
        }
    }

    pub fn children(&self) -> Vec<Self> {
        let mut children = Vec::new();
        self.gather_children(&mut children);
        children
    }

    pub fn filtered_children(&self, predicate: impl FnMut(&Self) -> bool) -> Vec<Self> {
        let mut children = Vec::new();
        self.gather_filtered_children(&mut children, predicate);
        children
    }

    fn subdivide(&mut self) {
        match self {
            CubeTreeNode::Leaf {
                half_size,
                face,
                bounds,
                ..
            } => {
                let center = bounds.center();
                let children = [
                    // Bottom left
                    Rectangle::from_corners(bounds.min, center),
                    // Bottom right
                    Rectangle::from_corners(
                        Vector2::new(center.x, bounds.min.y),
                        Vector2::new(bounds.max.x, center.y),
                    ),
                    // Top left
                    Rectangle::from_corners(
                        Vector2::new(bounds.min.x, center.y),
                        Vector2::new(center.x, bounds.max.y),
                    ),
                    // Top right
                    Rectangle::from_corners(center, bounds.max),
                ]
                .map(|child_bounds| {
                    Box::new(Self::Leaf {
                        collider: false,
                        half_size: *half_size,
                        face: *face,
                        bounds: child_bounds,
                    })
                });
                *self = CubeTreeNode::Internal {
                    bounds: *bounds,
                    children,
                }
            }
            _ => panic!("cannot subdivide an internal node"),
        };
    }
}

impl From<CubeTreeNode> for Rectangle {
    fn from(value: CubeTreeNode) -> Self {
        match value {
            CubeTreeNode::Internal { bounds, .. } => bounds,
            CubeTreeNode::Leaf { bounds, .. } => bounds,
        }
    }
}

impl PartialEq for CubeTreeNode {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Leaf { bounds, .. } => match other {
                CubeTreeNode::Leaf {
                    bounds: other_bounds,
                    ..
                } => bounds == other_bounds,
                _ => false,
            },
            Self::Internal {
                bounds, children, ..
            } => match other {
                CubeTreeNode::Internal {
                    bounds: other_bounds,
                    children: other_children,
                    ..
                } => {
                    if bounds != other_bounds {
                        return false;
                    }
                    for (child, other_child) in children.iter().zip(other_children) {
                        if child.bounds() != other_child.bounds() {
                            return false;
                        }
                    }
                    true
                }
                _ => false,
            },
        }
    }
}

impl std::fmt::Debug for CubeTreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CubeTreeNode::Internal { bounds, .. } => f.write_fmt(format_args!(
                "QuadTreeNode::Internal {{ (({},{}), ({},{}))}}\n",
                bounds.max.x, bounds.max.y, bounds.min.x, bounds.min.y
            )),
            CubeTreeNode::Leaf { bounds, .. } => f.write_fmt(format_args!(
                "QuadTreeNode::Leaf {{ (({},{}), ({},{})) }}\n",
                bounds.max.x, bounds.max.y, bounds.min.x, bounds.min.y
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_insert() {
        let radius = 100.0;
        let mut quad_cube = CubeTree::new(radius);
        quad_cube.insert(Vector::Z * radius * 2.0);

        let mut children = Vec::new();
        let z_face = quad_cube.get(Axis::Z);
        z_face.gather_children(&mut children);

        let expected = vec![];

        assert_eq!(children, expected);
    }
}
