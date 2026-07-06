use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::widget::NodeImageMode;

use crate::cards::CardAssets;
use crate::game::{
    AutoCompleteMsg, HintMsg, NewGameMsg, Session, UndoMsg, auto_complete_available,
};
use crate::theme;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct MovesText;

#[derive(Component)]
struct TimeText;

#[derive(Component)]
struct AutoButton;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct WinBanner;

#[derive(Component, Clone, Copy)]
enum Action {
    NewGame,
    Undo,
    Hint,
    AutoComplete,
}

pub fn plugin(app: &mut App) {
    insert_button_skin(app);
    app.add_systems(Startup, setup_ui).add_systems(
        Update,
        (
            update_hud,
            update_auto_button,
            update_win_banner,
            update_hud_visibility,
        ),
    );
}

fn update_hud_visibility(
    state: Res<State<crate::AppState>>,
    mut q: Query<
        &mut Visibility,
        Or<(
            With<HudRoot>,
            With<WinBanner>,
            With<crate::game::PileMarker>,
            With<crate::game::CardEntity>,
        )>,
    >,
) {
    let want = if *state.get() == crate::AppState::MainMenu {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut q {
        if *v != want {
            *v = want;
        }
    }
}

fn setup_ui(mut commands: Commands, assets: Res<CardAssets>, skin: Res<ButtonSkin>) {
    let font = assets.font.clone();
    let text_font = |size: f32| TextFont {
        font: font.clone().into(),
        font_size: size.into(),
        ..default()
    };

    // Bottom bar.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(52),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(px(18)),
                ..default()
            },
            HudRoot,
        ))
        .with_children(|bar| {
            bar.spawn(Node {
                column_gap: px(28),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|stats| {
                stats.spawn((
                    Text::new("SCORE 0"),
                    text_font(30.0),
                    TextColor(theme::GOLD),
                    text_shadow(),
                    ScoreText,
                ));
                stats.spawn((
                    Text::new("MOVES 0"),
                    text_font(30.0),
                    TextColor(theme::TEXT_DIM),
                    text_shadow(),
                    MovesText,
                ));
                stats.spawn((
                    Text::new("0:00"),
                    text_font(30.0),
                    TextColor(theme::TEXT_DIM),
                    text_shadow(),
                    TimeText,
                ));
            });

            bar.spawn(Node {
                column_gap: px(10),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|buttons| {
                spawn_button(
                    buttons,
                    &skin.small,
                    &font,
                    "AUTO",
                    theme::GREEN,
                    Action::AutoComplete,
                )
                .insert(AutoButton);
                spawn_button(
                    buttons,
                    &skin.small,
                    &font,
                    "HINT",
                    theme::BTN_BG,
                    Action::Hint,
                );
                spawn_button(
                    buttons,
                    &skin.small,
                    &font,
                    "UNDO",
                    theme::BTN_BG,
                    Action::Undo,
                );
                spawn_button(
                    buttons,
                    &skin.small,
                    &font,
                    "NEW",
                    theme::RED,
                    Action::NewGame,
                );
            });
        });

    // Win banner (hidden until won).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(120),
                left: px(0),
                right: px(0),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            WinBanner,
            Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("YOU WIN!"),
                text_font(96.0),
                TextColor(theme::GOLD),
                TextShadow {
                    offset: Vec2::splat(4.0),
                    color: theme::RED,
                },
            ));
        });
}

/// Soft drop shadow shared by HUD stats and button labels.
pub fn text_shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::splat(2.0),
        color: Color::srgba(0.0, 0.0, 0.0, 0.5),
    }
}

/// One button look: a flat white staircase-corner rect, 9-sliced and tinted
/// per button. The same image tinted translucent black doubles as the drop shadow.
#[derive(Clone)]
pub struct SkinImages {
    pub image: Handle<Image>,
    pub slicer: TextureSlicer,
    /// Shadow offset below the face, in px.
    pub drop: f32,
}

/// Two sizes: big for menu buttons (~60px tall), small for HUD/settings
/// buttons (~30-45px).
#[derive(Resource)]
pub struct ButtonSkin {
    pub big: SkinImages,
    pub small: SkinImages,
}

/// The tinted face layer of a button (hover recolors this child).
#[derive(Component)]
pub struct ButtonFace;

/// Flat white rect with 3-step staircase corners: cuts at 1/2/4 x `step` px.
/// No rim, no shadow — the tint does the coloring; a second node does the
/// shadow.
fn make_button_image(n: usize, step: usize) -> Image {
    let cut = |u: usize, v: usize| {
        (u < step && v < 4 * step) || (u < 2 * step && v < 2 * step) || (u < 4 * step && v < step)
    };

    let mut data = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (x.min(n - 1 - x), y.min(n - 1 - y));
            if !cut(u, v) {
                let i = (y * n + x) * 4;
                data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // Crisp steps regardless of the pixel-smoothing setting.
    image.sampler = ImageSampler::nearest();
    image
}

fn make_skin(images: &mut Assets<Image>, n: usize, step: usize, drop: f32) -> SkinImages {
    // Border covers the corner staircase (4*step) plus a straight-edge px so
    // corners render 1:1 and only straight edges stretch.
    let b = (4 * step + 2) as f32;
    SkinImages {
        image: images.add(make_button_image(n, step)),
        slicer: TextureSlicer {
            border: BorderRect::all(b),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 1.0,
        },
        drop,
    }
}

/// Insert ButtonSkin during plugin build so it exists before the initial
/// state transition (OnEnter(MainMenu) runs before Startup).
pub fn insert_button_skin(app: &mut App) {
    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    // "Chunky" per user: menu buttons 3px steps (12px corner, 6px shadow),
    // HUD/settings 2px steps (8px corner, 4px shadow).
    let big = make_skin(&mut images, 32, 3, 6.0);
    let small = make_skin(&mut images, 24, 2, 4.0);
    app.insert_resource(ButtonSkin { big, small });
}

/// Default button Node; override fields for bigger menu buttons. Paddings
/// carry +2px vs the old bordered look: the 2px border is gone, and border
/// size counted into the node box, so this keeps button footprints identical.
pub fn button_node() -> Node {
    Node {
        padding: UiRect::axes(px(16), px(6)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

/// A button's resting color; hover derives from this instead of mutating the
/// live color, so repeated hover cycles can't drift the hue.
#[derive(Component)]
pub struct BaseColor(pub Color);

/// Shared chunky-button look for HUD and menus. The root is a layout-only
/// node; visuals are two absolutely-positioned children (shadow behind face,
/// painted in child order) plus the in-flow label that defines the size.
pub fn button_visuals(
    skin: &SkinImages,
    font: &Handle<Font>,
    label: &str,
    bg: Color,
    font_size: f32,
    node: Node,
) -> impl Bundle {
    let sliced = ImageNode {
        image: skin.image.clone(),
        image_mode: NodeImageMode::Sliced(skin.slicer.clone()),
        ..default()
    };
    let fill = |top: f32| Node {
        position_type: PositionType::Absolute,
        left: px(0),
        right: px(0),
        top: px(top),
        bottom: px(-top),
        ..default()
    };
    (
        Button,
        node,
        BaseColor(bg),
        children![
            // Drop shadow: same staircase silhouette, black 30%, offset down.
            (
                fill(skin.drop),
                ImageNode {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.30),
                    ..sliced.clone()
                },
                Pickable::IGNORE,
            ),
            (
                fill(0.0),
                ImageNode {
                    color: bg,
                    ..sliced
                },
                ButtonFace,
                Pickable::IGNORE,
            ),
            (
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: font_size.into(),
                    ..default()
                },
                TextColor(theme::TEXT_LIGHT),
                text_shadow(),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// Lighten the face on hover, restore on out.
pub fn add_hover_effect(e: &mut EntityCommands) {
    e.observe(
        |over: On<Pointer<Over>>,
         buttons: Query<(&Children, &BaseColor), With<Button>>,
         mut faces: Query<&mut ImageNode, With<ButtonFace>>| {
            if let Ok((children, base)) = buttons.get(over.event_target()) {
                for child in children.iter() {
                    if let Ok(mut img) = faces.get_mut(child) {
                        img.color = base.0.lighter(0.08);
                    }
                }
            }
        },
    );
    e.observe(
        |out: On<Pointer<Out>>,
         buttons: Query<(&Children, &BaseColor), With<Button>>,
         mut faces: Query<&mut ImageNode, With<ButtonFace>>| {
            if let Ok((children, base)) = buttons.get(out.event_target()) {
                for child in children.iter() {
                    if let Ok(mut img) = faces.get_mut(child) {
                        img.color = base.0;
                    }
                }
            }
        },
    );
}

fn spawn_button<'a>(
    parent: &'a mut ChildSpawnerCommands<'_>,
    skin: &SkinImages,
    font: &Handle<Font>,
    label: &str,
    bg: Color,
    action: Action,
) -> EntityCommands<'a> {
    let mut e = parent.spawn((
        button_visuals(skin, font, label, bg, 26.0, button_node()),
        action,
    ));
    e.observe(
        move |click: On<Pointer<Click>>,
              mut new_game: MessageWriter<NewGameMsg>,
              mut undo: MessageWriter<UndoMsg>,
              mut hint: MessageWriter<HintMsg>,
              mut auto: MessageWriter<AutoCompleteMsg>| {
            let _ = &click;
            match action {
                Action::NewGame => {
                    new_game.write(NewGameMsg);
                }
                Action::Undo => {
                    undo.write(UndoMsg);
                }
                Action::Hint => {
                    hint.write(HintMsg);
                }
                Action::AutoComplete => {
                    auto.write(AutoCompleteMsg);
                }
            }
        },
    );
    add_hover_effect(&mut e);
    e
}

fn update_hud(
    session: Res<Session>,
    mut score: Query<&mut Text, (With<ScoreText>, Without<MovesText>, Without<TimeText>)>,
    mut moves: Query<&mut Text, (With<MovesText>, Without<ScoreText>, Without<TimeText>)>,
    mut time: Query<&mut Text, (With<TimeText>, Without<ScoreText>, Without<MovesText>)>,
) {
    if let Ok(mut t) = score.single_mut() {
        let new = format!("SCORE {}", session.score);
        if t.0 != new {
            t.0 = new;
        }
    }
    if let Ok(mut t) = moves.single_mut() {
        let new = format!("MOVES {}", session.engine.moves());
        if t.0 != new {
            t.0 = new;
        }
    }
    if let Ok(mut t) = time.single_mut() {
        let secs = session.elapsed as u32;
        let new = format!("{}:{:02}", secs / 60, secs % 60);
        if t.0 != new {
            t.0 = new;
        }
    }
}

fn update_auto_button(session: Res<Session>, mut q: Query<&mut Node, With<AutoButton>>) {
    if let Ok(mut node) = q.single_mut() {
        let show = auto_complete_available(session.engine.state()) && !session.auto;
        let want = if show { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
}

fn update_win_banner(session: Res<Session>, mut q: Query<&mut Node, With<WinBanner>>) {
    if let Ok(mut node) = q.single_mut() {
        let want = if session.won {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
    }
}
