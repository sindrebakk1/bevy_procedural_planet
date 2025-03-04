use avian3d::math::{Scalar, Vector2};
use bevy::input::ButtonInput;
use bevy::prelude::{Event, EventWriter, Gamepad, GamepadAxis, GamepadButton, KeyCode, Query, Res};

/// An event sent for a movement input action.
#[derive(Event)]
pub enum MovementAction {
    Move(Vector2),
    Jump,
}

/// Sends [`crate::plugins::player::kinematic_controller::MovementAction`] events based on keyboard input.
pub fn keyboard_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vector2::new(horizontal as Scalar, vertical as Scalar).clamp_length_max(1.0);

    if direction != Vector2::ZERO {
        movement_event_writer.send(MovementAction::Move(direction));
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_event_writer.send(MovementAction::Jump);
    }
}

/// Sends [`crate::plugins::player::kinematic_controller::MovementAction`] events based on gamepad input.
pub fn gamepad_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    gamepads: Query<&Gamepad>,
) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            movement_event_writer.send(MovementAction::Move(
                Vector2::new(x as Scalar, y as Scalar).clamp_length_max(1.0),
            ));
        }

        if gamepad.just_pressed(GamepadButton::South) {
            movement_event_writer.send(MovementAction::Jump);
        }
    }
}
