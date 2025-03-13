use avian3d::math::AdjustPrecision;

pub mod double;

#[cfg(feature = "f64")]
pub type Rectangle = double::d_rect::DRect;

#[cfg(not(feature = "f64"))]
pub type Rectangle = bevy::math::Rect;

#[cfg(feature = "f64")]
impl AdjustPrecision for double::d_rect::DRect {
    type Adjusted = Rectangle;

    fn adjust_precision(&self) -> Self::Adjusted {
        *self
    }

    #[cfg(not(feature = "f64"))]
    fn adjust_precision(&self) -> Self::Adjusted {
        self.as_rect()
    }
}
