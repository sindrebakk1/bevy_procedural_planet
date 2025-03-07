mod components;
mod backend;
mod controller;
mod walk;
mod action;
mod basis;
mod helpers;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
use bevy::utils::HashMap;

use components::{Motor, RigidBodyTracker};
use controller::{ActionFlowStatus, Controller};
use action::{ActionContext, ActionInitiationDirective, ActionLifecycleDirective, ActionLifecycleStatus};
use basis::BasisContext;
use components::{ProximitySensor, ControllerToggle};

/// Umbrella system set for [`PipelineStages`].
///
/// The physics backends' plugins are responsible for preventing this entire system set from
/// running when the physics backend itself is paused.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct CharacterControllerSystemSet;

/// The various stages of the  pipeline.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ControllerPipelineStages {
    /// Data is read from the physics backend.
    Sensors,
    /// Data is propagated through the subservient sensors.
    SubservientSensors,
    ///  decieds how the entity should be manipulated.
    Logic,
    /// Forces are applied in the physics backend.
    Motors,
}

#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct UserControlsSystemSet;

pub struct KinematicCharacterControllerPlugin {
    schedule: InternedScheduleLabel,
}

impl KinematicCharacterControllerPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Default for KinematicCharacterControllerPlugin {
    fn default() -> Self {
        Self::new(Update)
    }
}

impl Plugin for KinematicCharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            self.schedule,
            (
                ControllerPipelineStages::Sensors,
                ControllerPipelineStages::SubservientSensors,
                UserControlsSystemSet,
                ControllerPipelineStages::Logic,
                ControllerPipelineStages::Motors,
            )
                .chain()
                .in_set(CharacterControllerSystemSet),
        );
        app.add_systems(
            self.schedule,
            apply_controller_system.in_set(ControllerPipelineStages::Logic)
        )
    }
}

#[allow(clippy::type_complexity)]
fn apply_controller_system(
    time: Res<Time>,
    mut query: Query<(
        &mut Controller,
        &RigidBodyTracker,
        &mut ProximitySensor,
        &mut Motor,
        Option<&ControllerToggle>,
    )>,
) {
    let frame_duration = time.delta().as_secs_f32();
    if frame_duration == 0.0 {
        return;
    }
    for (mut controller, tracker, mut sensor, mut motor, tnua_toggle) in query.iter_mut() {
        match tnua_toggle.copied().unwrap_or_default() {
            ControllerToggle::Disabled => continue,
            ControllerToggle::SenseOnly => {}
            ControllerToggle::Enabled => {}
        }

        let controller = controller.as_mut();

        match controller.action_flow_status {
            ActionFlowStatus::NoAction | ActionFlowStatus::ActionOngoing(_) => {}
            ActionFlowStatus::ActionEnded(_) => {
                controller.action_flow_status = ActionFlowStatus::NoAction;
            }
            ActionFlowStatus::ActionStarted(action_name)
            | ActionFlowStatus::Cancelled {
                old: _,
                new: action_name,
            } => {
                controller.action_flow_status = ActionFlowStatus::ActionOngoing(action_name);
            }
        }

        if let Some((_, basis)) = controller.current_basis.as_mut() {
            let up_direction = Dir3::new(-tracker.gravity.f32()).unwrap_or(Dir3::Y);
            let basis = basis.as_mut();
            basis.apply(
                BasisContext {
                    frame_duration,
                    tracker,
                    proximity_sensor: sensor.as_ref(),
                    up_direction,
                },
                motor.as_mut(),
            );
            let sensor_cast_range_for_basis = basis.proximity_sensor_cast_range();

            // To streamline ActionContext creation
            let proximity_sensor = sensor.as_ref();

            let has_valid_contender = if let Some((_, contender_action, being_fed_for)) =
                &mut controller.contender_action
            {
                let initiation_decision = contender_action.initiation_decision(
                    ActionContext {
                        frame_duration,
                        tracker,
                        proximity_sensor,
                        basis,
                        up_direction,
                    },
                    being_fed_for,
                );
                being_fed_for.tick(time.delta());
                match initiation_decision {
                    ActionInitiationDirective::Reject => {
                        controller.contender_action = None;
                        false
                    }
                    ActionInitiationDirective::Delay => false,
                    ActionInitiationDirective::Allow => true,
                }
            } else {
                false
            };

            if let Some((name, current_action)) = controller.current_action.as_mut() {
                let lifecycle_status = if has_valid_contender {
                    ActionLifecycleStatus::CancelledInto
                } else if controller
                    .actions_being_fed
                    .get(name)
                    .map(|fed_entry| fed_entry.fed_this_frame)
                    .unwrap_or(false)
                {
                    ActionLifecycleStatus::StillFed
                } else {
                    ActionLifecycleStatus::NoLongerFed
                };

                let directive = current_action.apply(
                    ActionContext {
                        frame_duration,
                        tracker,
                        proximity_sensor,
                        basis,
                        up_direction,
                    },
                    lifecycle_status,
                    motor.as_mut(),
                );
                if current_action.violates_coyote_time() {
                    basis.violate_coyote_time();
                }
                let reschedule_action =
                    |actions_being_fed: &mut HashMap<&'static str, FedEntry>,
                     after_seconds: f32| {
                        if let Some(fed_entry) = actions_being_fed.get_mut(name) {
                            fed_entry.rescheduled_in =
                                Some(Timer::from_seconds(after_seconds, TimerMode::Once));
                        }
                    };
                match directive {
                    ActionLifecycleDirective::StillActive => {
                        if !lifecycle_status.is_active()
                            && matches!(
                                controller.action_flow_status,
                                ActionFlowStatus::ActionOngoing(_)
                            )
                        {
                            controller.action_flow_status = ActionFlowStatus::ActionEnded(name);
                        }
                    }
                    ActionLifecycleDirective::Finished
                    | ActionLifecycleDirective::Reschedule { .. } => {
                        if let ActionLifecycleDirective::Reschedule { after_seconds } =
                            directive
                        {
                            reschedule_action(&mut controller.actions_being_fed, after_seconds);
                        }
                        controller.current_action = if has_valid_contender {
                            let (contender_name, mut contender_action, _) = controller.contender_action.take().expect("has_valid_contender can only be true if contender_action is Some");
                            if let Some(contender_fed_entry) =
                                controller.actions_being_fed.get_mut(contender_name)
                            {
                                contender_fed_entry.rescheduled_in = None;
                            }
                            let contender_directive = contender_action.apply(
                                ActionContext {
                                    frame_duration,
                                    tracker,
                                    proximity_sensor,
                                    basis,
                                    up_direction,
                                },
                                ActionLifecycleStatus::CancelledFrom,
                                motor.as_mut(),
                            );
                            if contender_action.violates_coyote_time() {
                                basis.violate_coyote_time();
                            }
                            match contender_directive {
                                ActionLifecycleDirective::StillActive => {
                                    if matches!(
                                        controller.action_flow_status,
                                        ActionFlowStatus::ActionOngoing(_)
                                    ) {
                                        controller.action_flow_status =
                                            ActionFlowStatus::Cancelled {
                                                old: name,
                                                new: contender_name,
                                            };
                                    } else {
                                        controller.action_flow_status =
                                            ActionFlowStatus::ActionStarted(contender_name);
                                    }
                                    Some((contender_name, contender_action))
                                }
                                ActionLifecycleDirective::Finished => {
                                    if matches!(
                                        controller.action_flow_status,
                                        ActionFlowStatus::ActionOngoing(_)
                                    ) {
                                        controller.action_flow_status =
                                            ActionFlowStatus::ActionEnded(name);
                                    }
                                    None
                                }
                                ActionLifecycleDirective::Reschedule { after_seconds } => {
                                    if matches!(
                                        controller.action_flow_status,
                                        ActionFlowStatus::ActionOngoing(_)
                                    ) {
                                        controller.action_flow_status =
                                            ActionFlowStatus::ActionEnded(name);
                                    }
                                    reschedule_action(
                                        &mut controller.actions_being_fed,
                                        after_seconds,
                                    );
                                    None
                                }
                            }
                        } else {
                            controller.action_flow_status = ActionFlowStatus::ActionEnded(name);
                            None
                        };
                    }
                }
            } else if has_valid_contender {
                let (contender_name, mut contender_action, _) = controller
                    .contender_action
                    .take()
                    .expect("has_valid_contender can only be true if contender_action is Some");
                contender_action.apply(
                    ActionContext {
                        frame_duration,
                        tracker,
                        proximity_sensor,
                        basis,
                        up_direction,
                    },
                    ActionLifecycleStatus::Initiated,
                    motor.as_mut(),
                );
                if contender_action.violates_coyote_time() {
                    basis.violate_coyote_time();
                }
                controller.action_flow_status = ActionFlowStatus::ActionStarted(contender_name);
                controller.current_action = Some((contender_name, contender_action));
            }

            let sensor_case_range_for_action =
                if let Some((_, current_action)) = &controller.current_action {
                    current_action.proximity_sensor_cast_range()
                } else {
                    0.0
                };

            sensor.cast_range = sensor_cast_range_for_basis.max(sensor_case_range_for_action);
            sensor.cast_direction = -up_direction;
        }

        // Cycle actions_being_fed
        controller.actions_being_fed.retain(|_, fed_entry| {
            if fed_entry.fed_this_frame {
                fed_entry.fed_this_frame = false;
                if let Some(rescheduled_in) = &mut fed_entry.rescheduled_in {
                    rescheduled_in.tick(time.delta());
                }
                true
            } else {
                false
            }
        });

        if let Some((contender_name, ..)) = controller.contender_action {
            if !controller.actions_being_fed.contains_key(contender_name) {
                controller.contender_action = None;
            }
        }
    }
}
