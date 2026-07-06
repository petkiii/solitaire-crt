use bevy::prelude::*;

use crate::AppState;
use crate::cards::CardAssets;
use crate::game::NewGameMsg;
use crate::settings::Settings;
use crate::theme;
use crate::ui::{ButtonSkin, SkinImages, add_hover_effect, button_visuals};

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct PauseRoot;

#[derive(Component)]
struct SettingsRoot;

/// The single shared backdrop dim, shown while any overlay is open. Overlay
/// roots carry no dim of their own, so future stacked overlays (e.g. a
/// confirm prompt over the pause menu) won't double-darken the scene.
#[derive(Component)]
struct OverlayDim;

/// Whether the settings overlay is visible (on top of main menu or pause).
#[derive(Resource, Default)]
pub struct SettingsOpen(pub bool);

#[derive(Clone, Copy, PartialEq)]
enum SettingsField {
    Crt,
    Grain,
    Vignette,
    Animation,
}

impl SettingsField {
    const ALL: [SettingsField; 4] = [
        SettingsField::Crt,
        SettingsField::Grain,
        SettingsField::Vignette,
        SettingsField::Animation,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsField::Crt => "CRT",
            SettingsField::Grain => "GRAIN",
            SettingsField::Vignette => "VIGNETTE",
            SettingsField::Animation => "ANIM SPEED",
        }
    }

    fn min(self) -> f32 {
        match self {
            SettingsField::Animation => 0.5,
            _ => 0.0,
        }
    }

    fn max(self) -> f32 {
        match self {
            SettingsField::Animation => 2.0,
            _ => 1.0,
        }
    }

    fn get(self, s: &Settings) -> f32 {
        match self {
            SettingsField::Crt => s.crt,
            SettingsField::Grain => s.grain,
            SettingsField::Vignette => s.vignette,
            SettingsField::Animation => s.anim_speed,
        }
    }

    fn set(self, s: &mut Settings, v: f32) {
        let v = v.clamp(self.min(), self.max());
        match self {
            SettingsField::Crt => s.crt = v,
            SettingsField::Grain => s.grain = v,
            SettingsField::Vignette => s.vignette = v,
            SettingsField::Animation => s.anim_speed = v,
        }
    }

    /// Map a pointer position on the track to a snapped setting value.
    fn value_at(self, pointer_logical_x: f32, node: &ComputedNode, tf: &UiGlobalTransform) -> f32 {
        let px_x = pointer_logical_x / node.inverse_scale_factor();
        let width = node.size().x.max(1.0);
        let left = tf.translation.x - width * 0.5;
        let rel = ((px_x - left) / width).clamp(0.0, 1.0);
        let raw = self.min() + rel * (self.max() - self.min());
        (raw / 0.05).round() * 0.05
    }
}

#[derive(Component)]
struct SettingsValueText(SettingsField);

#[derive(Component)]
struct SliderTrack;

#[derive(Component)]
struct SliderFill(SettingsField);

#[derive(Component)]
struct EffectsToggleText;

#[derive(Component)]
struct BloomToggleText;

pub fn plugin(app: &mut App) {
    app.init_resource::<SettingsOpen>()
        .add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
        .add_systems(OnExit(AppState::MainMenu), despawn_all::<MainMenuRoot>)
        .add_systems(OnEnter(AppState::Paused), spawn_pause_menu)
        .add_systems(OnExit(AppState::Paused), despawn_all::<PauseRoot>)
        .add_systems(OnExit(AppState::Playing), clear_drag)
        .add_systems(
            Update,
            (handle_escape, sync_settings_panel, update_settings_texts),
        )
        .add_systems(Startup, spawn_settings_panel);
}

fn despawn_all<C: Component>(mut commands: Commands, q: Query<Entity, With<C>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn clear_drag(
    mut drag: ResMut<crate::input::DragState>,
    mut q: Query<&mut crate::animation::Moveable>,
) {
    if let Some(info) = drag.active.take() {
        for e in info.entities {
            if let Ok(mut m) = q.get_mut(e) {
                m.dragging = false;
            }
        }
    }
}

fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut settings_open: ResMut<SettingsOpen>,
    mut new_game: MessageWriter<NewGameMsg>,
) {
    // Enter on the title screen starts a game.
    if *state.get() == AppState::MainMenu && !settings_open.0 && keys.just_pressed(KeyCode::Enter) {
        new_game.write(NewGameMsg);
        next.set(AppState::Playing);
        return;
    }
    #[cfg(debug_assertions)]
    if keys.just_pressed(KeyCode::F10) {
        settings_open.0 = !settings_open.0;
        return;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if settings_open.0 {
        settings_open.0 = false;
        return;
    }
    match state.get() {
        AppState::Playing => next.set(AppState::Paused),
        AppState::Paused => next.set(AppState::Playing),
        AppState::MainMenu => {}
    }
}

fn menu_button<'a>(
    parent: &'a mut ChildSpawnerCommands<'_>,
    skin: &SkinImages,
    font: &Handle<Font>,
    label: &str,
    bg: Color,
) -> EntityCommands<'a> {
    let mut e = parent.spawn(button_visuals(
        skin,
        font,
        label,
        bg,
        34.0,
        Node {
            min_width: px(280),
            padding: UiRect::axes(px(22), px(10)),
            ..crate::ui::button_node()
        },
    ));
    add_hover_effect(&mut e);
    e
}

fn spawn_main_menu(mut commands: Commands, assets: Res<CardAssets>, skin: Res<ButtonSkin>) {
    let font = assets.font.clone();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(14),
                ..default()
            },
            MainMenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("SOLITAIRE"),
                TextFont {
                    font: font.clone().into(),
                    font_size: 120.0.into(),
                    ..default()
                },
                TextColor(theme::GOLD),
                TextShadow {
                    offset: Vec2::splat(5.0),
                    color: theme::RED,
                },
                Node {
                    margin: UiRect::bottom(px(40)),
                    ..default()
                },
                Pickable::IGNORE,
            ));

            menu_button(root, &skin.big, &font, "PLAY", theme::RED).observe(
                |_: On<Pointer<Click>>,
                 mut next: ResMut<NextState<AppState>>,
                 mut new_game: MessageWriter<NewGameMsg>| {
                    new_game.write(NewGameMsg);
                    next.set(AppState::Playing);
                },
            );
            menu_button(root, &skin.big, &font, "SETTINGS", theme::BTN_BG).observe(
                |_: On<Pointer<Click>>, mut open: ResMut<SettingsOpen>| {
                    open.0 = !open.0;
                },
            );
            menu_button(root, &skin.big, &font, "QUIT", theme::DARK).observe(
                |_: On<Pointer<Click>>,
                 settings: Res<Settings>,
                 mut exit: MessageWriter<AppExit>| {
                    settings.save();
                    exit.write(AppExit::Success);
                },
            );
        });
}

fn spawn_pause_menu(mut commands: Commands, assets: Res<CardAssets>, skin: Res<ButtonSkin>) {
    let font = assets.font.clone();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(12),
                ..default()
            },
            // Above the shared OverlayDim (5), below the settings panel (10).
            GlobalZIndex(6),
            PauseRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font: font.clone().into(),
                    font_size: 72.0.into(),
                    ..default()
                },
                TextColor(theme::GOLD),
                TextShadow {
                    offset: Vec2::splat(4.0),
                    color: theme::RED,
                },
                Node {
                    margin: UiRect::bottom(px(20)),
                    ..default()
                },
                Pickable::IGNORE,
            ));

            menu_button(root, &skin.big, &font, "RESUME", theme::GREEN).observe(
                |_: On<Pointer<Click>>, mut next: ResMut<NextState<AppState>>| {
                    next.set(AppState::Playing);
                },
            );
            menu_button(root, &skin.big, &font, "NEW GAME", theme::RED).observe(
                |_: On<Pointer<Click>>,
                 mut next: ResMut<NextState<AppState>>,
                 mut new_game: MessageWriter<NewGameMsg>| {
                    new_game.write(NewGameMsg);
                    next.set(AppState::Playing);
                },
            );
            menu_button(root, &skin.big, &font, "MAIN MENU", theme::BTN_BG).observe(
                |_: On<Pointer<Click>>, mut next: ResMut<NextState<AppState>>| {
                    next.set(AppState::MainMenu);
                },
            );
            menu_button(root, &skin.big, &font, "SETTINGS", theme::BTN_BG).observe(
                |_: On<Pointer<Click>>, mut open: ResMut<SettingsOpen>| {
                    open.0 = !open.0;
                },
            );
            menu_button(root, &skin.big, &font, "QUIT", theme::DARK).observe(
                |_: On<Pointer<Click>>,
                 settings: Res<Settings>,
                 mut exit: MessageWriter<AppExit>| {
                    settings.save();
                    exit.write(AppExit::Success);
                },
            );
        });
}

fn small_button<'a>(
    parent: &'a mut ChildSpawnerCommands<'_>,
    skin: &SkinImages,
    font: &Handle<Font>,
    label: &str,
) -> EntityCommands<'a> {
    let mut e = parent.spawn(button_visuals(
        skin,
        font,
        label,
        theme::L_DARK,
        24.0,
        Node {
            min_width: px(40),
            padding: UiRect::axes(px(12), px(4)),
            ..crate::ui::button_node()
        },
    ));
    add_hover_effect(&mut e);
    e
}

/// The settings panel exists once, hidden; shown over main menu or pause.
fn spawn_settings_panel(mut commands: Commands, assets: Res<CardAssets>, skin: Res<ButtonSkin>) {
    let font = assets.font.clone();
    let text_font = |size: f32| TextFont {
        font: font.clone().into(),
        font_size: size.into(),
        ..default()
    };

    // Shared backdrop dim for all overlays (see OverlayDim).
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            right: px(0),
            bottom: px(0),
            ..default()
        },
        GlobalZIndex(5),
        BackgroundColor(theme::OVERLAY_DIM),
        Visibility::Hidden,
        OverlayDim,
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(10),
            SettingsRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        row_gap: px(10),
                        padding: UiRect::all(px(28)),
                        border: UiRect::all(px(3)),
                        border_radius: BorderRadius::all(px(14)),
                        min_width: px(520),
                        ..default()
                    },
                    BackgroundColor(theme::PANEL_BG),
                    BorderColor::all(theme::L_DARK),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("SETTINGS"),
                        text_font(44.0),
                        TextColor(theme::GOLD),
                        Node {
                            align_self: AlignSelf::Center,
                            margin: UiRect::bottom(px(10)),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));

                    // Master effects toggle.
                    panel
                        .spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            column_gap: px(16),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("SCREEN EFFECTS"),
                                text_font(26.0),
                                TextColor(theme::TEXT_LIGHT),
                                Pickable::IGNORE,
                            ));
                            small_button(row, &skin.small, &font, "ON")
                                .insert(EffectsToggleText)
                                .observe(
                                    |_: On<Pointer<Click>>, mut settings: ResMut<Settings>| {
                                        settings.effects_enabled = !settings.effects_enabled;
                                    },
                                );
                        });

                    for field in SettingsField::ALL {
                        panel
                            .spawn(Node {
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                column_gap: px(16),
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn((
                                    Text::new(field.label()),
                                    text_font(26.0),
                                    TextColor(theme::TEXT_LIGHT),
                                    Pickable::IGNORE,
                                ));
                                row.spawn(Node {
                                    align_items: AlignItems::Center,
                                    column_gap: px(12),
                                    ..default()
                                })
                                .with_children(|controls| {
                                    let mut track = controls.spawn((
                                        Node {
                                            width: px(200),
                                            height: px(18),
                                            padding: UiRect::all(px(3)),
                                            border_radius: BorderRadius::all(px(9)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.0, 0.0, 0.05, 0.7)),
                                        SliderTrack,
                                    ));
                                    track.with_children(|t| {
                                        t.spawn((
                                            Node {
                                                width: percent(100),
                                                height: percent(100),
                                                border_radius: BorderRadius::all(px(6)),
                                                ..default()
                                            },
                                            BackgroundColor(theme::GOLD),
                                            SliderFill(field),
                                            Pickable::IGNORE,
                                        ));
                                    });
                                    let set_from = move |x: f32,
                                                         entity: Entity,
                                                         settings: &mut Settings,
                                                         q: &Query<
                                        (&ComputedNode, &UiGlobalTransform),
                                        With<SliderTrack>,
                                    >| {
                                        if let Ok((node, tf)) = q.get(entity) {
                                            field.set(settings, field.value_at(x, node, tf));
                                        }
                                    };
                                    track.observe(
                                        move |press: On<Pointer<Press>>,
                                              mut settings: ResMut<Settings>,
                                              q: Query<
                                            (&ComputedNode, &UiGlobalTransform),
                                            With<SliderTrack>,
                                        >| {
                                            set_from(
                                                press.pointer_location.position.x,
                                                press.event_target(),
                                                &mut settings,
                                                &q,
                                            );
                                        },
                                    );
                                    track.observe(
                                        move |drag: On<Pointer<Drag>>,
                                              mut settings: ResMut<Settings>,
                                              q: Query<
                                            (&ComputedNode, &UiGlobalTransform),
                                            With<SliderTrack>,
                                        >| {
                                            set_from(
                                                drag.pointer_location.position.x,
                                                drag.event_target(),
                                                &mut settings,
                                                &q,
                                            );
                                        },
                                    );
                                    controls.spawn((
                                        Text::new("100%"),
                                        text_font(26.0),
                                        TextColor(theme::GOLD),
                                        SettingsValueText(field),
                                        Node {
                                            min_width: px(80),
                                            justify_content: JustifyContent::Center,
                                            ..default()
                                        },
                                        Pickable::IGNORE,
                                    ));
                                });
                            });
                    }

                    // Bloom is an on/off post-process switch.
                    panel
                        .spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            column_gap: px(16),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("CRT BLOOM"),
                                text_font(26.0),
                                TextColor(theme::TEXT_LIGHT),
                                Pickable::IGNORE,
                            ));
                            small_button(row, &skin.small, &font, "ON")
                                .insert(BloomToggleText)
                                .observe(
                                    |_: On<Pointer<Click>>, mut settings: ResMut<Settings>| {
                                        settings.bloom_enabled = !settings.bloom_enabled;
                                    },
                                );
                        });

                    let mut back = panel.spawn(button_visuals(
                        &skin.small,
                        &font,
                        "BACK",
                        theme::RED,
                        28.0,
                        Node {
                            align_self: AlignSelf::Center,
                            margin: UiRect::top(px(14)),
                            padding: UiRect::axes(px(26), px(8)),
                            ..crate::ui::button_node()
                        },
                    ));
                    back.observe(|_: On<Pointer<Click>>, mut open: ResMut<SettingsOpen>| {
                        open.0 = false;
                    });
                    add_hover_effect(&mut back);
                });
        });
}

fn sync_settings_panel(
    open: Res<SettingsOpen>,
    state: Res<State<AppState>>,
    mut settings_root: Query<&mut Node, With<SettingsRoot>>,
    mut pause_root: Query<&mut Visibility, (With<PauseRoot>, Without<OverlayDim>)>,
    mut dim: Query<&mut Visibility, (With<OverlayDim>, Without<PauseRoot>)>,
) {
    if let Ok(mut node) = settings_root.single_mut() {
        let want = if open.0 { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
    // Settings replaces the pause menu (one visible overlay for now); a
    // future confirm-style prompt would instead stack on top of pause.
    let want = if open.0 {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut pause_root {
        if *v != want {
            *v = want;
        }
    }
    // One shared dim while any overlay is open, never stacked.
    let overlay_open = open.0 || *state.get() == AppState::Paused;
    let want = if overlay_open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in &mut dim {
        if *v != want {
            *v = want;
        }
    }
}

fn update_settings_texts(
    settings: Res<Settings>,
    mut values: Query<(&mut Text, &SettingsValueText)>,
    mut fills: Query<(&mut Node, &SliderFill)>,
    toggle_btn: Query<&Children, With<EffectsToggleText>>,
    bloom_btn: Query<&Children, With<BloomToggleText>>,
    mut texts: Query<&mut Text, Without<SettingsValueText>>,
) {
    if !settings.is_changed() {
        return;
    }
    for (mut text, value) in &mut values {
        let pct = (value.0.get(&settings) * 100.0).round() as i32;
        let new = format!("{pct}%");
        if text.0 != new {
            text.0 = new;
        }
    }
    for (mut node, fill) in &mut fills {
        let f = fill.0;
        let pct = (f.get(&settings) - f.min()) / (f.max() - f.min()) * 100.0;
        let want = percent(pct);
        if node.width != want {
            node.width = want;
        }
    }
    let mut sync_toggle = |children: &Children, on: bool| {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                let new = if on { "ON" } else { "OFF" };
                if text.0 != new {
                    text.0 = new.to_string();
                }
            }
        }
    };
    for children in &toggle_btn {
        sync_toggle(children, settings.effects_enabled);
    }
    for children in &bloom_btn {
        sync_toggle(children, settings.bloom_enabled);
    }
}
