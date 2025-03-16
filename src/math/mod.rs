use avian3d::math::AdjustPrecision;

pub mod double;
pub mod quad_tree;

#[cfg(feature = "f64")]
pub type Rectangle = double::d_rect::DRect;

#[cfg(feature = "f64")]
impl AdjustPrecision for double::d_rect::DRect {
    type Adjusted = Rectangle;

    fn adjust_precision(&self) -> Self::Adjusted {
        *self
    }
}

#[cfg(not(feature = "f64"))]
pub type Rectangle = bevy::math::Rect;
