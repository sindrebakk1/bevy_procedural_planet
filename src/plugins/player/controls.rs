use avian3d::math::{AdjustPrecision, Scalar, Vector};
use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_tnua::{
    builtins::{
        TnuaBuiltinCrouch, TnuaBuiltinCrouchState, TnuaBuiltinDash, TnuaBuiltinJump,
        TnuaBuiltinWalk,
    },
    control_helpers::{
        TnuaCrouchEnforcer, TnuaSimpleAirActionsCounter, TnuaSimpleFallThroughPlatformsHelper,
    },
    controller::TnuaController,
    TnuaAction, TnuaGhostSensor, TnuaProximitySensor,
};

use crate::plugins::physics::character_controller::{
    config::{
        CROUCH_FLOAT_OFFSET, DASH_DISTANCE, FLOAT_HEIGHT, JUMP_HEIGHT, MAX_SLOPE,
        ONE_WAY_PLATFORMS_MIN_PROXIMITY, SPEED, TURNING_ANGULAR_VELOCITY,
    },
    CharacterController,
};

#[allow(clippy::type_complexity)]
pub fn apply_player_controls(
    mut egui_context: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &mut TnuaController,
        &mut TnuaCrouchEnforcer,
        &mut TnuaProximitySensor,
        &TnuaGhostSensor,
        &mut TnuaSimpleFallThroughPlatformsHelper,
        &mut TnuaSimpleAirActionsCounter,
        Option<&ForwardFromCamera>,
    )>,
) {
    if egui_context.ctx_mut().wants_keyboard_input() {
        for (mut controller, ..) in query.iter_mut() {
            controller.neutralize_basis();
        }
        return;
    }

    for (
        mut controller,
        mut crouch_enforcer,
        mut sensor,
        ghost_sensor,
        mut fall_through_helper,
        mut air_actions_counter,
        forward_from_camera,
    ) in query.iter_mut()
    {
        let mut direction = Vec3::ZERO;

        if keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
            direction -= Vec3::Z;
        }
        if keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
            direction += Vec3::Z;
        }
        if keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
            direction -= Vec3::X;
        }
        if keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
            direction += Vec3::X;
        }

        direction = direction.clamp_length_max(1.0);

        if let Some(forward_from_camera) = forward_from_camera {
            direction = Transform::default()
                .looking_to(forward_from_camera.forward, Dir3::Y)
                .transform_point(direction)
        }

        let jump = keyboard.any_pressed([KeyCode::Space]);
        let dash = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

        let turn_in_place = forward_from_camera.is_none()
            && keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let crouch_buttons = [KeyCode::ControlLeft, KeyCode::ControlRight];
        let crouch_pressed = keyboard.any_pressed(crouch_buttons);
        let crouch_just_pressed = keyboard.any_just_pressed(crouch_buttons);

        air_actions_counter.update(controller.as_mut());

        let crouch;

        let mut handler =
            fall_through_helper.with(&mut sensor, ghost_sensor, ONE_WAY_PLATFORMS_MIN_PROXIMITY);
        if crouch_pressed {
            crouch = !handler.try_falling(crouch_just_pressed);
        } else {
            crouch = false;
            handler.dont_fall();
        }

        let speed_factor: Scalar =
            if let Some((_, state)) = controller.concrete_action::<TnuaBuiltinCrouch>() {
                if matches!(state, TnuaBuiltinCrouchState::Rising) {
                    1.0
                } else {
                    0.2
                }
            } else {
                1.0
            };

        controller.basis(TnuaBuiltinWalk {
            desired_velocity: if turn_in_place {
                Vector::ZERO
            } else {
                direction.adjust_precision() * speed_factor * SPEED
            },
            desired_forward: forward_from_camera
                .map_or(Dir3::new(direction).ok(), |camera| Some(camera.forward)),
            float_height: FLOAT_HEIGHT,
            max_slope: MAX_SLOPE,
            turning_angvel: TURNING_ANGULAR_VELOCITY,
            ..Default::default()
        });

        if crouch {
            controller.action(crouch_enforcer.enforcing(TnuaBuiltinCrouch {
                float_offset: CROUCH_FLOAT_OFFSET,
                ..Default::default()
            }));
        }

        if jump {
            controller.action(TnuaBuiltinJump {
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinJump::NAME) <= 1,
                height: JUMP_HEIGHT,
                ..Default::default()
            });
        }

        if dash {
            controller.action(TnuaBuiltinDash {
                displacement: direction.adjust_precision().normalize() * DASH_DISTANCE,
                desired_forward: if forward_from_camera.is_none() {
                    Dir3::new(direction).ok()
                } else {
                    None
                },
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinDash::NAME) <= 1,
                ..Default::default()
            });
        }
    }
}

pub fn apply_camera_controls(
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut player_character_query: Query<
        (&GlobalTransform, &mut ForwardFromCamera),
        With<CharacterController>,
    >,
    mut camera_query: Query<&mut Transform, With<PlayerCamera>>,
) {
    let mouse_controls_camera = primary_window_query
        .get_single()
        .map_or(false, |w| !w.cursor_options.visible);
    let total_delta = if mouse_controls_camera {
        mouse_motion.read().map(|event| event.delta).sum()
    } else {
        mouse_motion.clear();
        Vec2::ZERO
    };
    let Ok((player_transform, mut forward_from_camera)) = player_character_query.get_single_mut()
    else {
        return;
    };

    let yaw = Quat::from_rotation_y(-0.01 * total_delta.x);
    forward_from_camera.forward = Dir3::new_unchecked(
        yaw.mul_vec3(forward_from_camera.forward.as_vec3())
            .normalize(),
    );

    let pitch = 0.005 * total_delta.y;
    forward_from_camera.pitch_angle = (forward_from_camera.pitch_angle + pitch)
        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

    for mut camera in camera_query.iter_mut() {
        camera.translation =
            player_transform.translation() + -10.0 * forward_from_camera.forward + 1.0 * Vec3::Y;
        camera.look_to(forward_from_camera.forward, Vec3::Y);
        let pitch_axis = camera.left();
        camera.rotate_around(
            player_transform.translation(),
            Quat::from_axis_angle(*pitch_axis, forward_from_camera.pitch_angle),
        );
    }
}

pub fn grab_ungrab_mouse(
    mut egui_context: EguiContexts,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut primary_window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = primary_window_query.get_single_mut() else {
        return;
    };
    if window.cursor_options.visible {
        if mouse_buttons.just_pressed(MouseButton::Left) {
            if egui_context.ctx_mut().is_pointer_over_area() {
                return;
            }
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }
    } else if keyboard.just_released(KeyCode::Escape)
        || mouse_buttons.just_pressed(MouseButton::Left)
    {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

#[derive(Component, Default)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct ForwardFromCamera {
    pub forward: Dir3,
    pub pitch_angle: f32,
}

impl Default for ForwardFromCamera {
    fn default() -> Self {
        Self {
            forward: Dir3::NEG_Z,
            pitch_angle: 0.0,
        }
    }
}
