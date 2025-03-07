use super::action::Action;
use bevy::prelude::*;
use std::any::Any;

/// Various data passed to [`Basis::apply`].
pub struct BasisContext<'a> {
    /// The duration of the current frame.
    pub frame_duration: f32,

    /// A sensor that collects data about the rigid body from the physics backend.
    pub tracker: &'a RigidBodyTracker,

    /// A sensor that tracks the distance of the character's center from the ground.
    pub proximity_sensor: &'a ProximitySensor,

    /// The direction considered as "up".
    pub up_direction: Dir3,
}

/// The main movement command of a character.
///
/// A basis handles the character's motion when the user is not feeding it any input, or when it
/// just moves around without doing anything special. A simple game would only need once basis -
/// [`TnuaBuiltinWalk`](crate::builtins::TnuaBuiltinWalk) - but more complex games can have bases
/// for things like swimming or driving.
///
/// The type that implements this trait is called the basis _input_, and is expected to be
/// overwritten each frame by the controller system of the game code. Configuration is considered
/// as part of the input. If the basis needs to persist data between frames it must keep it in its
/// [state](Self::State).
pub trait Basis: 'static + Send + Sync {
    /// The default name of the basis.
    ///
    /// [Once `type_name` becomes `const`](https://github.com/rust-lang/rust/ihssues/63084), this
    /// will default to it. For now, just set it to the name of the type.
    const NAME: &'static str;

    /// Data that the basis can persist between frames.
    ///
    /// The basis will typically update this in its [`apply`](Self::apply). It has three purposes:
    ///
    /// 1. Store data that cannot be calculated on the spot. For example - a timer for tracking
    ///    coyote time.
    ///
    /// 2. Pass data from the basis to the action (or to Tnua's internal mechanisms)
    ///
    /// 3. Inspect the basis from game code systems, like an animation controlling system that
    ///    needs to know which animation to play based on the basis' current state.
    type State: Default + Send + Sync;

    /// This is where the basis affects the character's motion.
    ///
    /// This method gets called each frame to let the basis control the [`TnuaMotor`] that will
    /// later move the character.
    ///
    /// Note that after the motor is set in this method, if there is an action going on, the
    /// action's [`apply`](TnuaAction::apply) will also run and typically change some of the things
    /// the basis did to the motor.
    ///                                                              
    /// It can also update the state.
    fn apply(&self, state: &mut Self::State, ctx: BasisContext, motor: &mut Motor);

    /// A value to configure the range of the ground proximity sensor according to the basis'
    /// needs.
    fn proximity_sensor_cast_range(&self, state: &Self::State) -> f32;

    /// The displacement of the character from where the basis wants it to be.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn displacement(&self, state: &Self::State) -> Option<Vec3>;

    /// The velocity of the character, relative the what the basis considers its frame of
    /// reference.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn effective_velocity(&self, state: &Self::State) -> Vec3;

    /// The vertical velocity the character requires to stay the same height if it wants to move in
    /// [`effective_velocity`](Self::effective_velocity).
    fn vertical_velocity(&self, state: &Self::State) -> f32;

    /// Nullify the fields of the basis that represent user input.
    fn neutralize(&mut self);

    /// Can be queried by an action to determine if the character should be considered "in the air".
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn is_airborne(&self, state: &Self::State) -> bool;

    /// If the basis is at coyote time - finish the coyote time.
    ///
    /// This will be called automatically by Tnua, if the controller runs an action that  [violated
    /// coyote time](crate::plugins::player::kinematic_controller::action::TnuaAction::VIOLATES_COYOTE_TIME), so that a long coyote time will not allow,
    /// for example, unaccounted air jumps.
    ///
    /// If the character is fully grounded, this method must not change that.
    fn violate_coyote_time(&self, state: &mut Self::State);
}

/// Helper trait for accessing a basis and its trait with dynamic dispatch.
pub trait DynamicBasis: Send + Sync + Any + 'static {
    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any;

    #[doc(hidden)]
    fn as_mut_any(&mut self) -> &mut dyn Any;

    #[doc(hidden)]
    fn apply(&mut self, ctx: BasisContext, motor: &mut Motor);

    /// Dynamically invokes [`Basis::proximity_sensor_cast_range`].
    fn proximity_sensor_cast_range(&self) -> f32;

    /// Dynamically invokes [`Basis::displacement`].
    fn displacement(&self) -> Option<Vec3>;

    /// Dynamically invokes [`Basis::effective_velocity`].
    fn effective_velocity(&self) -> Vec3;

    /// Dynamically invokes [`Basis::vertical_velocity`].
    fn vertical_velocity(&self) -> f32;

    /// Dynamically invokes [`Basis::neutralize`].
    fn neutralize(&mut self);

    /// Dynamically invokes [`Basis::is_airborne`].
    fn is_airborne(&self) -> bool;

    #[doc(hidden)]
    fn violate_coyote_time(&mut self);
}

pub(crate) struct BoxableBasis<B: Basis> {
    pub(crate) input: B,
    pub(crate) state: B::State,
}

impl<B: Basis> BoxableBasis<B> {
    pub(crate) fn new(basis: B) -> Self {
        Self {
            input: basis,
            state: Default::default(),
        }
    }
}

impl<B: Basis> DynamicBasis for BoxableBasis<B> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(&mut self, ctx: BasisContext, motor: &mut Motor) {
        self.input.apply(&mut self.state, ctx, motor);
    }

    fn proximity_sensor_cast_range(&self) -> f32 {
        self.input.proximity_sensor_cast_range(&self.state)
    }

    fn displacement(&self) -> Option<Vec3> {
        self.input.displacement(&self.state)
    }

    fn effective_velocity(&self) -> Vec3 {
        self.input.effective_velocity(&self.state)
    }

    fn vertical_velocity(&self) -> f32 {
        self.input.vertical_velocity(&self.state)
    }

    fn neutralize(&mut self) {
        self.input.neutralize();
    }

    fn is_airborne(&self) -> bool {
        self.input.is_airborne(&self.state)
    }

    fn violate_coyote_time(&mut self) {
        self.input.violate_coyote_time(&mut self.state)
    }
}
