mod animation;
mod background;
mod cards;
mod deal;
mod engine;
mod game;
mod input;
mod layout;
mod menu;
mod post;
mod settings;
mod theme;
mod ui;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    Playing,
    Paused,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Solitaire".into(),
                        resolution: (1280, 800).into(),
                        // Hidden until the background/CRT shaders are ready;
                        // background::reveal_window flips it (with a timeout).
                        visible: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(theme::BG))
        .init_state::<AppState>()
        .add_plugins((
            settings::plugin,
            game::plugin,
            animation::plugin,
            input::plugin,
            ui::plugin,
            menu::plugin,
            background::plugin,
            post::plugin,
        ))
        .run();
}
