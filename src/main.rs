#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::app::PluginGroup;
use bevy::window::{Window, WindowPlugin};
use bevy::{
    app::{App, AppExit},
    DefaultPlugins,
};
use procedural_planet::GamePlugin;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins::set(
            DefaultPlugins,
            WindowPlugin {
                primary_window: Some(Window {
                    title: "Procedural Planet".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ))
        .add_plugins(GamePlugin)
        .run()
}
