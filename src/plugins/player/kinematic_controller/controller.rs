use bevy::prelude::{Component, Timer};
use bevy::time::Stopwatch;
use bevy::utils::{Entry, HashMap};
use crate::plugins::player::kinematic_controller::action::{Action, BoxableAction, DynamicAction};
use crate::plugins::player::kinematic_controller::basis::{Basis, BoxableBasis, DynamicBasis};

pub struct FedEntry {
    fed_this_frame: bool,
    rescheduled_in: Option<Timer>,
}

/// The main component used for interaction with the controls and animation code.
///
/// Every frame, the game code should feed input this component on every controlled entity. What
/// should be fed is:
///
/// * A basis - this is the main movement command - usually
///   [`BuiltinWalk`](crate::builtins::BuiltinWalk), but there can be others. It is the
///   game code's responsibility to ensure only one basis is fed at any given time, because basis
///   can hold state and replacing the basis type restarts the state.
///
///   Refer to the documentation of [the implementors of
///   `Basis`](crate::Basis#implementors) for more information.
///
/// * Zero or more actions - these are movements like jumping, dashing, crouching, etc. Multiple
///   actions can be fed, but only one can be active at any given moment. Unlike basis, there is a
///   smart mechanism for deciding which action to use and which to discard, so it is safe to feed
///   many actions at the same frame.
///
///   Refer to the documentation of [the implementors of
///   `Action`](crate::Action#implementors) for more information.
///
/// Without [`ControllerPlugin`] this component will not do anything.
#[derive(Component, Default)]
#[require(Motor, RigidBodyTracker, ProximitySensor)]
pub struct Controller {
    current_basis: Option<(&'static str, Box<dyn DynamicBasis>)>,
    actions_being_fed: HashMap<&'static str, FedEntry>,
    current_action: Option<(&'static str, Box<dyn DynamicAction>)>,
    contender_action: Option<(&'static str, Box<dyn DynamicAction>, Stopwatch)>,
    action_flow_status: ActionFlowStatus,
}

impl Controller {
    /// Feed a basis - the main movement command - with [its default name](Basis::NAME).
    pub fn basis<B: Basis>(&mut self, basis: B) {
        self.named_basis(B::NAME, basis);
    }

    /// Feed a basis - the main movement command - with a custom name.
    ///
    /// This should only be used if the same basis type needs to be used with different names to
    /// allow, for example, different animations. Otherwise prefer to use the default name with
    /// [`basis`](Self::basis).
    pub fn named_basis<B: Basis>(&mut self, name: &'static str, basis: B) {
        if let Some((existing_name, existing_basis)) =
            self.current_basis.as_mut().and_then(|(n, b)| {
                let b = b.as_mut_any().downcast_mut::<BoxableBasis<B>>()?;
                Some((n, b))
            })
        {
            *existing_name = name;
            existing_basis.input = basis;
        } else {
            self.current_basis = Some((name, Box::new(BoxableBasis::new(basis))));
        }
    }

    /// Instruct the basis to pretend the user provided no input this frame.
    ///
    /// The exact meaning is defined in the basis' [`neutralize`](Basis::neutralize) method,
    /// but generally it means that fields that typically come from a configuration will not be
    /// touched, and only fields that are typically set by user input get nullified.
    pub fn neutralize_basis(&mut self) {
        if let Some((_, basis)) = self.current_basis.as_mut() {
            basis.neutralize();
        }
    }

    /// The name of the currently running basis.
    ///
    /// When using the basis with it's default name, prefer to match this against
    /// [`Basis::NAME`] and not against a string literal.
    pub fn basis_name(&self) -> Option<&'static str> {
        self.current_basis
            .as_ref()
            .map(|(basis_name, _)| *basis_name)
    }

    /// A dynamic accessor to the currently running basis.
    pub fn dynamic_basis(&self) -> Option<&dyn DynamicBasis> {
        Some(self.current_basis.as_ref()?.1.as_ref())
    }

    /// The currently running basis, together with its state.
    ///
    /// This is mainly useful for animation. When multiple basis types are used in the game,
    /// [`basis_name`](Self::basis_name) be used to determine the type of the current basis first,
    /// to avoid having to try multiple downcasts.
    pub fn concrete_basis<B: Basis>(&self) -> Option<(&B, &B::State)> {
        let (_, basis) = self.current_basis.as_ref()?;
        let boxable_basis: &BoxableBasis<B> = basis.as_any().downcast_ref()?;
        Some((&boxable_basis.input, &boxable_basis.state))
    }

    /// Feed an action with [its default name](Basis::NAME).
    pub fn action<A: Action>(&mut self, action: A) {
        self.named_action(A::NAME, action);
    }

    /// Feed an action with a custom name.
    ///
    /// This should only be used if the same action type needs to be used with different names to
    /// allow, for example, different animations. Otherwise prefer to use the default name with
    /// [`action`](Self::action).
    pub fn named_action<A: Action>(&mut self, name: &'static str, action: A) {
        match self.actions_being_fed.entry(name) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().fed_this_frame = true;
                if let Some((current_name, current_action)) = self.current_action.as_mut() {
                    if *current_name == name {
                        let Some(current_action) = current_action
                            .as_mut_any()
                            .downcast_mut::<BoxableAction<A>>()
                        else {
                            panic!("Multiple action types registered with same name {name:?}");
                        };
                        current_action.input = action;
                    } else {
                        // different action is running - will not override because button was
                        // already pressed.
                    }
                } else if self.contender_action.is_none()
                    && entry
                    .get()
                    .rescheduled_in
                    .as_ref()
                    .map_or(false, |timer| timer.finished())
                {
                    // no action is running - but this action is rescheduled and there is no
                    // already-existing contender that would have taken priority
                    self.contender_action =
                        Some((name, Box::new(BoxableAction::new(action)), Stopwatch::new()));
                } else {
                    // no action is running - will not set because button was already pressed.
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(FedEntry {
                    fed_this_frame: true,
                    rescheduled_in: None,
                });
                if let Some(contender_action) = self.contender_action.as_mut().and_then(
                    |(contender_name, contender_action, _)| {
                        if *contender_name == name {
                            let Some(contender_action) = contender_action
                                .as_mut_any()
                                .downcast_mut::<BoxableAction<A>>()
                            else {
                                panic!("Multiple action types registered with same name {name:?}");
                            };
                            Some(contender_action)
                        } else {
                            None
                        }
                    },
                ) {
                    contender_action.input = action;
                } else {
                    self.contender_action =
                        Some((name, Box::new(BoxableAction::new(action)), Stopwatch::new()));
                }
            }
        }
    }

    /// The name of the currently running action.
    ///
    /// When using an action with it's default name, prefer to match this against
    /// [`Action::NAME`] and not against a string literal.
    pub fn action_name(&self) -> Option<&'static str> {
        self.current_action
            .as_ref()
            .map(|(action_name, _)| *action_name)
    }

    /// A dynamic accessor to the currently running action.
    pub fn dynamic_action(&self) -> Option<&dyn DynamicAction> {
        Some(self.current_action.as_ref()?.1.as_ref())
    }

    /// The currently running action, together with its state.
    ///
    /// This is mainly useful for animation. When multiple action types are used in the game,
    /// [`action_name`](Self::action_name) be used to determine the type of the current action
    /// first, to avoid having to try multiple downcasts.
    pub fn concrete_action<A: Action>(&self) -> Option<(&A, &A::State)> {
        let (_, action) = self.current_action.as_ref()?;
        let boxable_action: &BoxableAction<A> = action.as_any().downcast_ref()?;
        Some((&boxable_action.input, &boxable_action.state))
    }

    /// Indicator for the state and flow of movement actions.
    ///
    /// Query this every frame to keep track of the actions. For air actions,
    /// [`AirActionsTracker`](crate::control_helpers::AirActionsTracker) is easier to use
    /// (and uses this behind the scenes)
    ///
    /// The benefits of this over querying [`action_name`](Self::action_name) every frame are:
    ///
    /// * `action_flow_status` can indicate when the same action has been fed again immediately
    ///   after stopping or cancelled into itself.
    /// * `action_flow_status` shows an [`ActionEnded`](ActionFlowStatus::ActionEnded) when the
    ///   action is no longer fed, even if the action is still active (termination sequence)
    pub fn action_flow_status(&self) -> &ActionFlowStatus {
        &self.action_flow_status
    }

    /// Checks if the character is currently airborne.
    ///
    /// The check is done based on the basis, and is equivalent to getting the controller's
    /// [`dynamic_basis`](Self::dynamic_basis) and checking its
    /// [`is_airborne`](Basis::is_airborne) method.
    pub fn is_airborne(&self) -> Result<bool, ControllerHasNoBasis> {
        match self.dynamic_basis() {
            Some(basis) => Ok(basis.is_airborne()),
            None => Err(ControllerHasNoBasis),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("The  controller does not have any basis set")]
pub struct ControllerHasNoBasis;

/// The result of [`Controller::action_flow_status()`].
#[derive(Debug, Default, Clone)]
pub enum ActionFlowStatus {
    /// No action is going on.
    #[default]
    NoAction,

    /// An action just started.
    ActionStarted(&'static str),

    /// An action was fed in a past frame and is still ongoing.
    ActionOngoing(&'static str),

    /// An action has stopped being fed.
    ///
    /// Note that the action may still have a termination sequence after this happens.
    ActionEnded(&'static str),

    /// An action has just been canceled into another action.
    Cancelled {
        old: &'static str,
        new: &'static str,
    },
}

impl ActionFlowStatus {
    /// The name of the ongoing action, if there is an ongoing action.
    ///
    /// Will also return a value if the action has just started.
    pub fn ongoing(&self) -> Option<&'static str> {
        match self {
            ActionFlowStatus::NoAction | ActionFlowStatus::ActionEnded(_) => None,
            ActionFlowStatus::ActionStarted(action_name)
            | ActionFlowStatus::ActionOngoing(action_name)
            | ActionFlowStatus::Cancelled {
                old: _,
                new: action_name,
            } => Some(action_name),
        }
    }

    /// The name of the action that has just started this frame.
    ///
    /// Will return `None` if there is no action, or if the ongoing action has started in a past
    /// frame.
    pub fn just_starting(&self) -> Option<&'static str> {
        match self {
            ActionFlowStatus::NoAction
            | ActionFlowStatus::ActionOngoing(_)
            | ActionFlowStatus::ActionEnded(_) => None,
            ActionFlowStatus::ActionStarted(action_name)
            | ActionFlowStatus::Cancelled {
                old: _,
                new: action_name,
            } => Some(action_name),
        }
    }
}
