use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::ecs::schedule::{ScheduleConfigs, ScheduleLabel};
use bevy::ecs::system::BoxedSystem;
use bevy::math::Vec2Swizzles;
use bevy::picking::PickingSystems;
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

use crate::settings::Settings;

/// Screen-space CRT-style pass: curvature, edge fade, color fringing,
/// scanline overlay, grain, bloom, and vignette. Lives on the Camera2d;
/// strengths come from Settings each frame.
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
pub struct CrtPost {
    pub time: f32,
    pub crt: f32,
    pub grain: f32,
    pub vignette: f32,
    /// 0/1 bloom switch.
    pub bloom: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl FullscreenMaterial for CrtPost {
    fn fragment_shader() -> ShaderRef {
        "shaders/crt.wgsl".into()
    }

    fn schedule() -> impl ScheduleLabel + Clone {
        bevy::core_pipeline::Core2d
    }

    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        // After the UI pass so HUD/menus get the CRT treatment too (ui_pass
        // itself runs after Core2dSystems::PostProcess and before upscaling).
        system
            .after(bevy::ui_render::ui_pass)
            .before(bevy::core_pipeline::upscaling::upscaling)
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins(FullscreenMaterialPlugin::<CrtPost>::default())
        .add_systems(Update, drive)
        .add_systems(
            PreUpdate,
            // PointerLocation is written from winit input in ProcessInput;
            // the picking backends consume it in Backend. Warp in between so
            // every hit test sees the remapped position.
            remap_pointer_for_crt
                .after(PickingSystems::ProcessInput)
                .before(PickingSystems::Backend),
        );
}

fn drive(time: Res<Time>, settings: Res<Settings>, mut q: Query<&mut CrtPost>) {
    let (crt, grain, vignette, bloom) = settings.fx();
    for mut post in &mut q {
        post.time = time.elapsed_secs();
        post.crt = crt;
        post.grain = grain;
        post.vignette = vignette;
        post.bloom = bloom;
    }
}

/// CPU mirror of crt.wgsl's tube warp: the scene point that is VISIBLE at
/// output position `pos` (the shader samples the scene at the warped uv).
/// MUST stay in sync with the shader's zoom + cross-axis bulge.
pub fn crt_warp(pos: Vec2, window: Vec2, c: f32) -> Vec2 {
    if c <= 0.0 || window.x <= 0.0 || window.y <= 0.0 {
        return pos;
    }
    let uv = pos / window; // logical px -> uv, y-down on both sides
    let mut p = uv * 2.0 - 1.0;
    p *= 1.0 - 0.006 * c;
    p += p.yx() * p.yx() * p * Vec2::new(0.038, 0.054) * c;
    (p * 0.5 + 0.5) * window
}

/// The CRT pass bends the image, so what the OS cursor visually points at is
/// not the raw pixel under it. Rewrite the mouse pointer's picking position
/// through the same warp; raw input is tracked here so the transform is
/// always applied to fresh coordinates, never to an already-warped value.
fn remap_pointer_for_crt(
    mut moved: MessageReader<CursorMoved>,
    mut raw: Local<Option<Vec2>>,
    windows: Query<&Window>,
    settings: Res<Settings>,
    mut pointers: Query<(&PointerId, &mut PointerLocation)>,
) {
    for ev in moved.read() {
        *raw = Some(ev.position);
    }
    let Some(raw_pos) = *raw else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    // Same strength the shader receives (settings.fx().0 -> settings.crt).
    let warped = crt_warp(
        raw_pos,
        Vec2::new(window.width(), window.height()),
        settings.fx().0,
    );
    for (id, mut ploc) in &mut pointers {
        if *id != PointerId::Mouse {
            continue;
        }
        // Skip while inactive (cursor outside the window) and skip writes
        // that wouldn't change anything (change-detection churn on idle).
        if ploc.location.as_ref().is_some_and(|l| l.position != warped)
            && let Some(loc) = ploc.location.as_mut()
        {
            loc.position = warped;
        }
    }
}
