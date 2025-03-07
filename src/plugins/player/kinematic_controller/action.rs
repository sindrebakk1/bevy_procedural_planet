use bevy::prelude::*;
use bevy::time::Stopwatch;

use crate::plugins::player::kinematic_controller::basis::{
    Basis, BasisContext, BoxableBasis, DynamicBasis,
};
use crate::plugins::player::kinematic_controller::components::{
    Motor, ProximitySensor, RigidBodyTracker,
};
use std::{any::Any, time::Duration};

/// Various data passed to [`Action::apply`].
pub struct ActionContext<'a> {
    /// The duration of the current frame.
    pub frame_duration: f32,

    /// A sensor that collects data about the rigid body from the physics backend.
    pub tracker: &'a RigidBodyTracker,

    /// A sensor that tracks the distance of the character's center from the ground.
    pub proximity_sensor: &'a ProximitySensor,

    /// The direction considered as "up".
    pub up_direction: Dir3,

    /// An accessor to the currently active basis.
    pub basis: &'a dyn DynamicBasis,
}

impl<'a> ActionContext<'a> {
    /// Can be used to get the concrete basis.
    ///
    /// Use with care - actions that use it will only be usable with one basis.
    pub fn concrete_basis<B: Basis>(&self) -> Option<(&B, &B::State)> {
        let boxable_basis: &BoxableBasis<B> = self.basis.as_any().downcast_ref()?;
        Some((&boxable_basis.input, &boxable_basis.state))
    }

    /// "Downgrade" to a basis context.
    ///
    /// This is useful for some helper methods of [the concrete basis and its
    /// state](Self::concrete_basis) that require a basis context.
    pub fn as_basis_context(&self) -> BasisContext<'a> {
        BasisContext {
            frame_duration: self.frame_duration,
            tracker: self.tracker,
            proximity_sensor: self.proximity_sensor,
            up_direction: self.up_direction,
        }
    }

    pub fn frame_duration_as_duration(&self) -> Duration {
        Duration::from_secs_f64(self.frame_duration.into())
    }
}

/// Input for [`Action::apply`] that informs it about the long-term feeding of the input.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ActionLifecycleStatus {
    /// There was no action in the previous frame
    Initiated,
    /// There was a different action in the previous frame
    CancelledFrom,
    /// This action was already active in the previous frame, and it keeps getting fed
    StillFed,
    /// This action was fed up until the previous frame, and now no action is fed
    NoLongerFed,
    /// This action was fed up until the previous frame, and now a different action tries to override it
    CancelledInto,
}

impl ActionLifecycleStatus {
    /// Continue if the action is still fed, finish if its not fed or if some other action gets
    /// fed.
    pub fn directive_simple(&self) -> ActionLifecycleDirective {
        match self {
            ActionLifecycleStatus::Initiated => ActionLifecycleDirective::StillActive,
            ActionLifecycleStatus::CancelledFrom => ActionLifecycleDirective::StillActive,
            ActionLifecycleStatus::StillFed => ActionLifecycleDirective::StillActive,
            ActionLifecycleStatus::NoLongerFed => ActionLifecycleDirective::Finished,
            ActionLifecycleStatus::CancelledInto => ActionLifecycleDirective::Finished,
        }
    }

    /// Similar to [`directive_simple`](Self::directive_simple), but if some other action gets fed
    /// and this action is still being fed, reschedule this action once the other action finishes,
    /// as long as more time than `after_seconds` has passed.
    pub fn directive_simple_reschedule(&self, after_seconds: f32) -> ActionLifecycleDirective {
        match self {
            ActionLifecycleStatus::Initiated => ActionLifecycleDirective::StillActive,
            ActionLifecycleStatus::CancelledFrom => ActionLifecycleDirective::StillActive,
            ActionLifecycleStatus::StillFed => ActionLifecycleDirective::StillActive,
            ActionLifecycleStatus::NoLongerFed => {
                // The rescheduling will probably go away, but in case things happen too fast and
                // it doesn't - pass it anyway.
                ActionLifecycleDirective::Reschedule { after_seconds }
            }
            ActionLifecycleStatus::CancelledInto => {
                ActionLifecycleDirective::Reschedule { after_seconds }
            }
        }
    }

    /// Determine if the action just started, whether from no action or to replace another action.
    pub fn just_started(&self) -> bool {
        match self {
            ActionLifecycleStatus::Initiated => true,
            ActionLifecycleStatus::CancelledFrom => true,
            ActionLifecycleStatus::StillFed => false,
            ActionLifecycleStatus::NoLongerFed => false,
            ActionLifecycleStatus::CancelledInto => false,
        }
    }

    /// Determine if the action is currently active - still fed and not replaced by another.
    pub fn is_active(&self) -> bool {
        match self {
            ActionLifecycleStatus::Initiated => true,
            ActionLifecycleStatus::CancelledFrom => true,
            ActionLifecycleStatus::StillFed => true,
            ActionLifecycleStatus::NoLongerFed => false,
            ActionLifecycleStatus::CancelledInto => false,
        }
    }
}

/// A decision by [`Action::apply`] that determines if the action should be continued or not.
///
/// Note that an action may continue (probably with different state) after no longer being fed, or
/// stopped while still being fed. It's up to the action, and it should be responsible with it.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ActionLifecycleDirective {
    /// The action should continue in the next frame.
    StillActive,

    /// The action should not continue in the next frame.
    ///
    /// If another action is pending, it will run in this frame. This means that two actions can
    /// run in the same frame, as long as the first is finished (or
    /// [rescheduled](Self::Reschedule))
    ///
    /// If [`Action::apply`] returns this but the action is still being fed, it will not run
    /// again unless it stops being fed for one frame and then gets fed again. If this is not the
    /// desired behavior, [`ActionLifecycleDirective::Reschedule`] should be used instead.
    Finished,

    /// The action should not continue in the next frame, but if its still being fed it run again
    /// later. The rescheduled action will be considered a new action.
    ///
    /// If another action is pending, it will run in this frame. This means that two actions can
    /// run in the same frame, as long as the first is rescheduled (or [finished](Self::Finished))
    Reschedule {
        /// Only reschedule the action after this much time has passed.
        after_seconds: f32,
    },
}

/// A decision by [`Action::initiation_decision`] that determines if the action can start.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ActionInitiationDirective {
    /// The action will not start as long as the input is still fed. In order to start it, the
    /// input must be released for at least one frame and then start being fed again.
    Reject,

    /// The action will not start this frame, but if the input is still fed next frame
    /// [`Action::initiation_decision`] will be checked again.
    Delay,

    /// The action can start this frame.
    Allow,
}

/// A character movement command for performing special actions.
///
/// "Special" does not necessarily mean **that** special - even
/// [jumping](crate::builtins::BuiltinJump) or [crouching](crate::builtins::BuiltinCrouch)
/// are considered [`Action`]s. Unlike basis - which is something constant - an action is
/// usually something more momentarily that has a flow.
///
/// The type that implements this trait is called the action _input_, and is expected to be
/// overwritten each frame by the controller system of the game code - although unlike basis the
/// input will probably be the exact same. Configuration is considered as part of the input. If the
/// action needs to persist data between frames it must keep it in its [state](Self::State).
pub trait Action: 'static + Send + Sync {
    /// The default name of the action.
    ///
    /// [Once `type_name` becomes `const`](https://github.com/rust-lang/rust/issues/63084), this
    /// will default to it. For now, just set it to the name of the type.
    const NAME: &'static str;

    /// Data that the action can persist between frames.
    ///
    /// The action will typically update this in its [`apply`](Self::apply). It has three purposes:
    ///
    /// 1. Store data that cannot be calculated on the spot. For example - the part of the jump
    ///    the character is currently at.
    ///
    /// 2. Pass data from the action to 's internal mechanisms.
    ///
    /// 3. Inspect the action from game code systems, like an animation controlling system that
    ///    needs to know which animation to play based on the action's current state.
    type State: Default + Send + Sync;

    /// Set this to true for actions that may launch the character into the air.
    const VIOLATES_COYOTE_TIME: bool;

    /// This is where the action affects the character's motion.
    ///
    /// This method gets called each frame to let the action control the [`Motor`] that will
    /// later move the character. Note that this happens the motor was set by the basis'
    /// [`apply`](Basis::apply). Here the action can modify some aspects of or even completely
    /// overwrite what the basis did.
    ///                                                              
    /// It can also update the state.
    ///
    /// The returned value of this action determines whether or not the action will continue in the
    /// next frame.
    fn apply(
        &self,
        state: &mut Self::State,
        ctx: ActionContext,
        lifecycle_status: ActionLifecycleStatus,
        motor: &mut Motor,
    ) -> ActionLifecycleDirective;

    /// A value to configure the range of the ground proximity sensor according to the action's
    /// needs.
    fn proximity_sensor_cast_range(&self) -> f32 {
        0.0
    }

    /// Decides whether the action can start.
    ///
    /// The difference between rejecting the action here with
    /// [`ActionInitiationDirective::Reject`] or [`ActionInitiationDirective::Delay`] and
    /// approving it with [`ActionInitiationDirective::Allow`] only to do nothing in it and
    /// terminate with [`ActionLifecycleDirective::Finished`] on the first frame, is that if
    /// some other action is currently running, in the former that action will continue to be
    /// active, while in the latter it'll be cancelled into this new action - which, having being
    /// immediately finished, will leave the controller with no active action, or with some third
    /// action if there is one.
    fn initiation_decision(
        &self,
        ctx: ActionContext,
        being_fed_for: &Stopwatch,
    ) -> ActionInitiationDirective;
}

pub trait DynamicAction: Send + Sync + Any + 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn apply(
        &mut self,
        ctx: ActionContext,
        lifecycle_status: ActionLifecycleStatus,
        motor: &mut Motor,
    ) -> ActionLifecycleDirective;
    fn proximity_sensor_cast_range(&self) -> f32;
    fn initiation_decision(
        &self,
        ctx: ActionContext,
        being_fed_for: &Stopwatch,
    ) -> ActionInitiationDirective;
    fn violates_coyote_time(&self) -> bool;
}

pub(crate) struct BoxableAction<A: Action> {
    pub(crate) input: A,
    pub(crate) state: A::State,
}

impl<A: Action> BoxableAction<A> {
    pub(crate) fn new(basis: A) -> Self {
        Self {
            input: basis,
            state: Default::default(),
        }
    }
}

impl<A: Action> DynamicAction for BoxableAction<A> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(
        &mut self,
        ctx: ActionContext,
        lifecycle_status: ActionLifecycleStatus,
        motor: &mut Motor,
    ) -> ActionLifecycleDirective {
        self.input
            .apply(&mut self.state, ctx, lifecycle_status, motor)
    }

    fn proximity_sensor_cast_range(&self) -> f32 {
        self.input.proximity_sensor_cast_range()
    }

    fn initiation_decision(
        &self,
        ctx: ActionContext,
        being_fed_for: &Stopwatch,
    ) -> ActionInitiationDirective {
        self.input.initiation_decision(ctx, being_fed_for)
    }

    fn violates_coyote_time(&self) -> bool {
        A::VIOLATES_COYOTE_TIME
    }
}
