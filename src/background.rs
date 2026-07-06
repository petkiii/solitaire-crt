use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, CachedPipelineState, PipelineCache, ShaderType};
use bevy::render::{ExtractSchedule, MainWorld, RenderApp};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{Material2d, Material2dPlugin};

use crate::AppState;
use crate::layout::{VIRTUAL_H, VIRTUAL_W};

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct SwirlParams {
    pub colour_1: Vec4,
    pub colour_2: Vec4,
    pub colour_3: Vec4,
    pub resolution: Vec2,
    pub time: f32,
    pub spin_time: f32,
    pub contrast: f32,
    pub spin_amount: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct SwirlMaterial {
    #[uniform(0)]
    pub params: SwirlParams,
}

impl Material2d for SwirlMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/background.wgsl".into()
    }
}

/// Per-state colour targets for the animated backdrop.
struct BgPreset {
    c1: Vec4,
    c2: Vec4,
    c3: Vec4,
    contrast: f32,
    spin: f32,
}

fn linear(color: Color) -> Vec4 {
    color.to_linear().to_vec4()
}

fn preset(state: AppState) -> BgPreset {
    match state {
        // Title screen: high-contrast blue/red glow.
        AppState::MainMenu => BgPreset {
            c1: linear(crate::theme::BLUE),
            c2: linear(crate::theme::RED),
            c3: linear(Color::srgb(0.08, 0.09, 0.16)),
            contrast: 3.5,
            spin: 1.0,
        },
        // In-game: softer green table glow.
        AppState::Playing | AppState::Paused => BgPreset {
            c1: linear(Color::srgb(0.408, 0.673, 0.561)),
            c2: linear(Color::srgb(0.282, 0.466, 0.388)),
            c3: linear(Color::srgb(0.220, 0.362, 0.302)),
            contrast: 1.0,
            spin: 0.2,
        },
    }
}

#[derive(Resource)]
struct SwirlHandle(Handle<SwirlMaterial>);

const QUAD_W: f32 = VIRTUAL_W * 2.0;
const QUAD_H: f32 = VIRTUAL_H * 2.0;

pub fn plugin(app: &mut App) {
    app.add_plugins(Material2dPlugin::<SwirlMaterial>::default())
        .init_resource::<PipelinesPending>()
        .add_systems(Startup, setup)
        .add_systems(Update, (animate, reveal_window));
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, track_pending_pipelines);
}

/// Render pipelines still compiling (Queued/Creating), mirrored from the
/// render world each frame. Asset load state alone isn't enough to know the
/// backdrop can draw: pipeline compilation is async and much slower in dev
/// builds, so the window reveal below waits on the real signal.
#[derive(Resource, Default)]
struct PipelinesPending(usize);

fn track_pending_pipelines(cache: Res<PipelineCache>, mut main_world: ResMut<MainWorld>) {
    let pending = cache
        .pipelines()
        .filter(|p| {
            matches!(
                p.state,
                CachedPipelineState::Queued | CachedPipelineState::Creating(_)
            )
        })
        .count();
    let mut res = main_world.resource_mut::<PipelinesPending>();
    if res.0 != pending {
        res.0 = pending;
    }
}

/// The window starts hidden (main.rs) so its first visible frame already has
/// the swirl + CRT passes; otherwise bare ClearColor shows while the WGSL
/// loads from disk and the pipelines compile.
///
/// Reveal = both shader assets loaded, then a settling window of frames
/// (main-world load state leads the render world by several frames: the
/// asset still has to be extracted before the material specializes and
/// QUEUES its pipeline — `pending` can read 0 before our pipelines even
/// exist), and finally nothing left compiling. A timeout keeps a shader
/// failure from leaving the window invisible forever.
fn reveal_window(
    mut windows: Query<&mut Window>,
    server: Res<AssetServer>,
    pending: Res<PipelinesPending>,
    time: Res<Time<Real>>,
    mut shaders: Local<Option<(Handle<Shader>, Handle<Shader>)>>,
    mut frames_since_loaded: Local<u32>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    if window.visible {
        return;
    }
    // Same asset paths the materials use — the server dedups to one load.
    // The handles live in the Local so the shaders can't be dropped.
    let (bg, crt) = shaders.get_or_insert_with(|| {
        (
            server.load("shaders/background.wgsl"),
            server.load("shaders/crt.wgsl"),
        )
    });
    if server.is_loaded(bg.id()) && server.is_loaded(crt.id()) {
        *frames_since_loaded += 1;
    }
    // NOTE: in release the first mapped frame has the full backdrop. Dev
    // builds may still flash ClearColor for ~0.3s after mapping: Wayland
    // surfaces can't present while hidden, and the post-map warmup is slow
    // unoptimized — that part is beyond what this gate can control.
    let settled = *frames_since_loaded >= 10 && pending.0 == 0;
    if settled || time.elapsed_secs() > 5.0 {
        window.visible = true;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SwirlMaterial>>,
) {
    let p = preset(AppState::MainMenu);
    let handle = materials.add(SwirlMaterial {
        params: SwirlParams {
            colour_1: p.c1,
            colour_2: p.c2,
            colour_3: p.c3,
            resolution: Vec2::new(QUAD_W, QUAD_H),
            time: 0.0,
            spin_time: 0.0,
            contrast: p.contrast,
            spin_amount: p.spin,
            _pad0: 0.0,
            _pad1: 0.0,
        },
    });

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(QUAD_W, QUAD_H))),
        MeshMaterial2d(handle.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
        Pickable::IGNORE,
    ));
    commands.insert_resource(SwirlHandle(handle));
}

fn animate(
    time: Res<Time>,
    state: Res<State<AppState>>,
    handle: Res<SwirlHandle>,
    mut materials: ResMut<Assets<SwirlMaterial>>,
) {
    let Some(mut material) = materials.get_mut(&handle.0) else {
        return;
    };
    let dt = time.delta_secs();
    let target = preset(*state.get());
    // ~1s ease toward the state's colour preset.
    let k = 1.0 - (-3.0 * dt).exp();

    let p = &mut material.params;
    p.time += dt;
    p.spin_time += dt;
    p.colour_1 = p.colour_1.lerp(target.c1, k);
    p.colour_2 = p.colour_2.lerp(target.c2, k);
    p.colour_3 = p.colour_3.lerp(target.c3, k);
    p.contrast += (target.contrast - p.contrast) * k;
    p.spin_amount += (target.spin - p.spin_amount) * k;
}
