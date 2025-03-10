#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    app::{App, AppExit, PluginGroup},
    window::{Window, WindowPlugin},
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
