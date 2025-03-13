use avian3d::math::Scalar;

pub const SPEED: Scalar = 20.0;
pub const FLOAT_HEIGHT: Scalar = 2.0;
#[cfg(feature = "f64")]
pub const MAX_SLOPE: Scalar = std::f64::consts::FRAC_PI_4;
#[cfg(feature = "f32")]
pub const MAX_SLOPE: Scalar = std::f32::consts::FRAC_PI_4;
pub const TURNING_ANGULAR_VELOCITY: Scalar = Scalar::INFINITY;
pub const ACTIONS_IN_AIR: usize = 1;
pub const JUMP_HEIGHT: Scalar = 4.0;
pub const CROUCH_FLOAT_OFFSET: Scalar = -0.9;
pub const DASH_DISTANCE: Scalar = 10.0;
pub const ONE_WAY_PLATFORMS_MIN_PROXIMITY: Scalar = 1.0;
