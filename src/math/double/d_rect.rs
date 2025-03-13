use bevy::{
    math::{DVec2, IRect, Rect, URect},
    reflect::Reflect,
};

#[derive(Debug, Copy, Clone, Reflect)]
pub struct DRect {
    /// The minimum corner point of the rect.
    pub min: DVec2,
    /// The maximum corner point of the rect.
    pub max: DVec2,
}

#[allow(unused)]
impl DRect {
    /// An empty `DRect`, represented by maximum and minimum corner points
    /// at `DVec2::NEG_INFINITY` and `DVec2::INFINITY`, respectively.
    /// This is so the `DRect` has a infinitely negative size.
    /// This is useful, because when taking a union B of a non-empty `DRect` A and
    /// this empty `DRect`, B will simply equal A.
    pub const EMPTY: Self = Self {
        max: DVec2::NEG_INFINITY,
        min: DVec2::INFINITY,
    };
    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// use procedural_planet::math::double::DRect;
    ///
    /// let r = DRect::new(0., 4., 10., 6.); // w=10 h=2
    /// let r = DRect::new(2., 3., 5., -1.); // w=3 h=4
    /// ```
    #[inline]
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self::from_corners(DVec2::new(x0, y0), DVec2::new(x1, y1))
    }

    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy::math::{DVec2};
    /// use procedural_planet::math::double::DRect;
    ///
    /// // Unit rect from [0,0] to [1,1]
    /// let r = DRect::from_corners(DVec2::ZERO, DVec2::ONE); // w=1 h=1
    /// // Same; the points do not need to be ordered
    /// let r = DRect::from_corners(DVec2::ONE, DVec2::ZERO); // w=1 h=1
    /// ```
    #[inline]
    pub fn from_corners(p0: DVec2, p1: DVec2) -> Self {
        Self {
            min: p0.min(p1),
            max: p0.max(p1),
        }
    }

    /// Create a new rectangle from its center and size.
    ///
    /// # Panics
    ///
    /// This method panics if any of the components of the size is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::from_center_size(DVec2::ZERO, DVec2::ONE); // w=1 h=1
    /// assert!(r.min.abs_diff_eq(DVec2::splat(-0.5), 1e-5));
    /// assert!(r.max.abs_diff_eq(DVec2::splat(0.5), 1e-5));
    /// ```
    #[inline]
    pub fn from_center_size(origin: DVec2, size: DVec2) -> Self {
        assert!(size.cmpge(DVec2::ZERO).all(), "Rect size must be positive");
        let half_size = size / 2.;
        Self::from_center_half_size(origin, half_size)
    }

    /// Create a new rectangle from its center and half-size.
    ///
    /// # Panics
    ///
    /// This method panics if any of the components of the half-size is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::DVec2;
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::from_center_half_size(DVec2::ZERO, DVec2::ONE); // w=2 h=2
    /// assert!(r.min.abs_diff_eq(DVec2::splat(-1.), 1e-5));
    /// assert!(r.max.abs_diff_eq(DVec2::splat(1.), 1e-5));
    /// ```
    #[inline]
    pub fn from_center_half_size(origin: DVec2, half_size: DVec2) -> Self {
        assert!(
            half_size.cmpge(DVec2::ZERO).all(),
            "Rect half_size must be positive"
        );
        Self {
            min: origin - half_size,
            max: origin + half_size,
        }
    }

    /// Check if the rectangle is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::from_corners(DVec2::ZERO, DVec2::new(0., 1.)); // w=0 h=1
    /// assert!(r.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.cmpge(self.max).any()
    }

    /// Rectangle width (max.x - min.x).
    ///
    /// # Examples
    ///
    /// ```
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!((r.width() - 5.).abs() <= 1e-5);
    /// ```
    #[inline]
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    /// Rectangle height (max.y - min.y).
    ///
    /// # Examples
    ///
    /// ```
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!((r.height() - 1.).abs() <= 1e-5);
    /// ```
    #[inline]
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    /// Rectangle size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.size().abs_diff_eq(DVec2::new(5., 1.), 1e-5));
    /// ```
    #[inline]
    pub fn size(&self) -> DVec2 {
        self.max - self.min
    }

    /// Rectangle half-size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.half_size().abs_diff_eq(DVec2::new(2.5, 0.5), 1e-5));
    /// ```
    #[inline]
    pub fn half_size(&self) -> DVec2 {
        self.size() * 0.5
    }

    /// The center point of the rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.center().abs_diff_eq(DVec2::new(2.5, 0.5), 1e-5));
    /// ```
    #[inline]
    pub fn center(&self) -> DVec2 {
        (self.min + self.max) * 0.5
    }

    /// Check if a point lies within this rectangle, inclusive of its edges.
    ///
    /// # Examples
    ///
    /// ```
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.contains(r.center()));
    /// assert!(r.contains(r.min));
    /// assert!(r.contains(r.max));
    /// ```
    #[inline]
    pub fn contains(&self, point: DVec2) -> bool {
        (point.cmpge(self.min) & point.cmple(self.max)).all()
    }

    /// Build a new rectangle formed of the union of this rectangle and another rectangle.
    ///
    /// The union is the smallest rectangle enclosing both rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r1 = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// let r2 = DRect::new(1., -1., 3., 3.); // w=2 h=4
    /// let r = r1.union(r2);
    /// assert!(r.min.abs_diff_eq(DVec2::new(0., -1.), 1e-5));
    /// assert!(r.max.abs_diff_eq(DVec2::new(5., 3.), 1e-5));
    /// ```
    #[inline]
    pub fn union(&self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Build a new rectangle formed of the union of this rectangle and a point.
    ///
    /// The union is the smallest rectangle enclosing both the rectangle and the point. If the
    /// point is already inside the rectangle, this method returns a copy of the rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// let u = r.union_point(DVec2::new(3., 6.));
    /// assert!(u.min.abs_diff_eq(DVec2::ZERO, 1e-5));
    /// assert!(u.max.abs_diff_eq(DVec2::new(5., 6.), 1e-5));
    /// ```
    #[inline]
    pub fn union_point(&self, other: DVec2) -> Self {
        Self {
            min: self.min.min(other),
            max: self.max.max(other),
        }
    }

    /// Build a new rectangle formed of the intersection of this rectangle and another rectangle.
    ///
    /// The intersection is the largest rectangle enclosed in both rectangles. If the intersection
    /// is empty, this method returns an empty rectangle ([`bevy::math::Rect::is_empty()`] returns `true`), but
    /// the actual values of [`bevy::math::Rect::min`] and [`bevy::math::Rect::max`] are implementation-dependent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r1 = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// let r2 = DRect::new(1., -1., 3., 3.); // w=2 h=4
    /// let r = r1.intersect(r2);
    /// assert!(r.min.abs_diff_eq(DVec2::new(1., 0.), 1e-5));
    /// assert!(r.max.abs_diff_eq(DVec2::new(3., 1.), 1e-5));
    /// ```
    #[inline]
    pub fn intersect(&self, other: Self) -> Self {
        let mut r = Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        };
        // Collapse min over max to enforce invariants and ensure e.g. width() or
        // height() never return a negative value.
        r.min = r.min.min(r.max);
        r
    }

    /// Create a new rectangle by expanding it evenly on all sides.
    ///
    /// A positive expansion value produces a larger rectangle,
    /// while a negative expansion value produces a smaller rectangle.
    /// If this would result in zero or negative width or height, [`bevy::math::Rect::EMPTY`] is returned instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(0., 0., 5., 1.); // w=5 h=1
    /// let r2 = r.inflate(3.); // w=11 h=7
    /// assert!(r2.min.abs_diff_eq(DVec2::splat(-3.), 1e-5));
    /// assert!(r2.max.abs_diff_eq(DVec2::new(8., 4.), 1e-5));
    ///
    /// let r = DRect::new(0., -1., 6., 7.); // w=6 h=8
    /// let r2 = r.inflate(-2.); // w=11 h=7
    /// assert!(r2.min.abs_diff_eq(DVec2::new(2., 1.), 1e-5));
    /// assert!(r2.max.abs_diff_eq(DVec2::new(4., 5.), 1e-5));
    /// ```
    #[inline]
    pub fn inflate(&self, expansion: f64) -> Self {
        let mut r = Self {
            min: self.min - expansion,
            max: self.max + expansion,
        };
        // Collapse min over max to enforce invariants and ensure e.g. width() or
        // height() never return a negative value.
        r.min = r.min.min(r.max);
        r
    }

    /// Build a new rectangle from this one with its coordinates expressed
    /// relative to `other` in a normalized ([0..1] x [0..1]) coordinate system.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy::math::{DVec2};
    /// # use procedural_planet::math::double::DRect;
    /// let r = DRect::new(2., 3., 4., 6.);
    /// let s = DRect::new(0., 0., 10., 10.);
    /// let n = r.normalize(s);
    ///
    /// assert_eq!(n.min.x, 0.2);
    /// assert_eq!(n.min.y, 0.3);
    /// assert_eq!(n.max.x, 0.4);
    /// assert_eq!(n.max.y, 0.6);
    /// ```
    pub fn normalize(&self, other: Self) -> Self {
        let outer_size = other.size();
        Self {
            min: (self.min - other.min) / outer_size,
            max: (self.max - other.min) / outer_size,
        }
    }

    /// Returns self as [`Rect`] (i32)
    #[inline]
    pub fn as_rect(&self) -> Rect {
        Rect::from_corners(self.min.as_vec2(), self.max.as_vec2())
    }

    /// Returns self as [`IRect`] (i32)
    #[inline]
    pub fn as_irect(&self) -> IRect {
        IRect::from_corners(self.min.as_ivec2(), self.max.as_ivec2())
    }

    /// Returns self as [`URect`] (u32)
    #[inline]
    pub fn as_urect(&self) -> URect {
        URect::from_corners(self.min.as_uvec2(), self.max.as_uvec2())
    }
}

impl From<DRect> for Rect {
    fn from(value: DRect) -> Self {
        value.as_rect()
    }
}

impl From<Rect> for DRect {
    fn from(value: Rect) -> Self {
        DRect::from_corners(value.min.as_dvec2(), value.max.as_dvec2())
    }
}

impl PartialEq for DRect {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}
