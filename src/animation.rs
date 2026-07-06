use bevy::prelude::*;

use crate::cards::{CardAssets, set_face};
use crate::engine::card::Card;
use crate::layout::{SCALE, VIRTUAL_H};

/// Spring movement: the transform (VT) chases a target (T) with velocity,
/// picking up tilt from horizontal velocity. Constants are tuned in virtual
/// units and converted at 100 px per unit.
pub const PX_PER_UNIT: f32 = 100.0;

#[derive(Component)]
pub struct Moveable {
    pub target: Vec2,
    pub z: f32,
    pub vel: Vec2,
    /// Relative scale on top of the global sprite SCALE; 1.0 at rest.
    pub scale: f32,
    pub scale_vel: f32,
    pub rot: f32,
    pub rot_vel: f32,
    pub hover: bool,
    pub dragging: bool,
    /// Seconds before the spring engages (used for the staggered deal).
    pub delay: f32,
}

impl Moveable {
    pub fn at(pos: Vec2, z: f32) -> Self {
        Self {
            target: pos,
            z,
            vel: Vec2::ZERO,
            scale: 1.0,
            scale_vel: 0.0,
            rot: 0.0,
            rot_vel: 0.0,
            hover: false,
            dragging: false,
            delay: 0.0,
        }
    }

    /// Loose thresholds: "close enough" that a flip can start while the card
    /// is still gliding the last few pixels in.
    pub fn is_settled(&self, transform: &Transform) -> bool {
        self.delay <= 0.0
            && (transform.translation.truncate() - self.target).length_squared() < 144.0
            && self.vel.length_squared() < 9.0
    }
}

/// Decaying sine pulse on scale and rotation.
#[derive(Component)]
pub struct Juice {
    pub t: f32,
    pub amount: f32,
    pub r_amount: f32,
}

impl Juice {
    pub const DURATION: f32 = 0.4;

    pub fn new(amount: f32) -> Self {
        let sign = if rand::random::<bool>() { 1.0 } else { -1.0 };
        Self {
            t: 0.0,
            amount,
            r_amount: 0.6 * amount * sign,
        }
    }
}

/// Horizontal flip: scale.x shrinks to 0, face swaps, grows back.
#[derive(Component)]
pub struct Flip {
    pub t: f32,
    pub card: Card,
    pub face_up: bool,
    pub swapped: bool,
}

impl Flip {
    pub const DURATION: f32 = 0.15;
}

/// Gold pulse used for hints; removed after `t` reaches DURATION.
#[derive(Component)]
pub struct Hinted {
    pub t: f32,
}

impl Hinted {
    pub const DURATION: f32 = 1.2;
}

/// Win-celebration ballistic card; takes over from Moveable.
#[derive(Component)]
pub struct Cascading {
    pub vel: Vec2,
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            tick_juice,
            tick_flip,
            tick_hint,
            move_moveables,
            move_cascading,
        )
            .chain(),
    );
}

/// Fixed simulation step for the card integrator. The per-frame spring
/// formulas are not all frame-rate invariant (the rot/scale spring gains
/// carry no dt scaling, so their stiffness grows with fps and they ring at
/// uncapped frame rates), so the math must always see the same dt: real
/// frame time accumulates and the springs advance in fixed quanta. Must stay
/// above the highest target display refresh rate or motion will judder.
const SUBSTEP: f32 = 1.0 / 240.0;

fn move_moveables(
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
    mut acc: Local<f32>,
    mut q: Query<
        (&mut Transform, &mut Moveable, Option<&Juice>, Option<&Flip>),
        Without<Cascading>,
    >,
) {
    // The cap mirrors the old per-frame dt clamp: at most ~33ms of
    // simulation per rendered frame, excess dropped on hitches.
    *acc = (*acc + time.delta_secs() * settings.anim_speed).min(1.0 / 30.0);

    let dt = SUBSTEP;
    let k_xy = (-50.0 * dt).exp();
    let k_scale = (-60.0 * dt).exp();
    let k_r = (-190.0 * dt).exp();
    let max_vel = 70.0 * dt * PX_PER_UNIT;

    while *acc >= SUBSTEP {
        *acc -= SUBSTEP;
        step_moveables(dt, k_xy, k_scale, k_r, max_vel, &mut q);
    }
}

fn step_moveables(
    dt: f32,
    k_xy: f32,
    k_scale: f32,
    k_r: f32,
    max_vel: f32,
    q: &mut Query<
        (&mut Transform, &mut Moveable, Option<&Juice>, Option<&Flip>),
        Without<Cascading>,
    >,
) {
    for (mut transform, mut m, juice, flip) in q {
        if m.delay > 0.0 {
            m.delay -= dt;
            continue;
        }

        let vt = transform.translation.truncate();
        let mut vel = m.vel;
        vel = k_xy * vel + (1.0 - k_xy) * (m.target - vt) * 35.0 * dt;
        if vel.length_squared() > max_vel * max_vel {
            vel = vel.normalize() * max_vel;
        }
        let mut new_vt = vt + vel;
        if (new_vt - m.target).length_squared() < 0.01 && vel.length_squared() < 0.01 {
            new_vt = m.target;
            vel = Vec2::ZERO;
        }
        m.vel = vel;

        // Tilt from horizontal velocity.
        let des_r = (-0.015 * (vel.x / PX_PER_UNIT) / dt).clamp(-0.35, 0.35)
            + juice.map(|j| j.rot()).unwrap_or(0.0) * 2.0;
        m.rot_vel = k_r * m.rot_vel + (1.0 - k_r) * (des_r - m.rot);
        m.rot += m.rot_vel;

        let des_scale = 1.0
            + if m.dragging { 0.1 } else { 0.0 }
            + if m.hover { 0.05 } else { 0.0 }
            + juice.map(|j| j.scale()).unwrap_or(0.0);
        m.scale_vel = k_scale * m.scale_vel + (1.0 - k_scale) * (des_scale - m.scale);
        m.scale += m.scale_vel;

        let moving = (new_vt - m.target).length_squared() > 16.0 || m.dragging;
        let z = m.z + if moving { 300.0 } else { 0.0 };

        let flip_x = flip.map(|f| f.scale_x()).unwrap_or(1.0);
        transform.translation = new_vt.extend(z);
        transform.rotation = Quat::from_rotation_z(m.rot);
        transform.scale = Vec3::new(SCALE * m.scale * flip_x, SCALE * m.scale, 1.0);
    }
}

impl Juice {
    pub fn scale(&self) -> f32 {
        let decay = (1.0 - self.t / Self::DURATION).max(0.0);
        self.amount * (50.8 * self.t).sin() * decay.powi(3)
    }

    pub fn rot(&self) -> f32 {
        let decay = (1.0 - self.t / Self::DURATION).max(0.0);
        self.r_amount * (40.8 * self.t).sin() * decay.powi(2)
    }
}

fn tick_juice(
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Juice)>,
) {
    for (entity, mut juice) in &mut q {
        juice.t += time.delta_secs() * settings.anim_speed;
        if juice.t >= Juice::DURATION {
            commands.entity(entity).remove::<Juice>();
        }
    }
}

impl Flip {
    pub fn scale_x(&self) -> f32 {
        let half = Self::DURATION / 2.0;
        if self.t < half {
            1.0 - self.t / half
        } else {
            (self.t - half) / half
        }
    }
}

fn tick_flip(
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
    mut commands: Commands,
    assets: Option<Res<CardAssets>>,
    mut q: Query<(Entity, &mut Flip, &mut Sprite, &crate::game::CardEntity)>,
    mut art_q: Query<
        (&mut Sprite, &mut Visibility),
        (
            With<crate::cards::FaceArt>,
            Without<crate::game::CardEntity>,
        ),
    >,
) {
    let Some(assets) = assets else { return };
    for (entity, mut flip, mut sprite, ce) in &mut q {
        flip.t += time.delta_secs() * settings.anim_speed;
        if !flip.swapped
            && flip.t >= Flip::DURATION / 2.0
            && let Ok((mut art, mut art_vis)) = art_q.get_mut(ce.art)
        {
            flip.swapped = true;
            set_face(
                &mut sprite,
                &mut art,
                &mut art_vis,
                &assets,
                &flip.card,
                flip.face_up,
            );
        }
        if flip.t >= Flip::DURATION {
            commands.entity(entity).remove::<Flip>();
        }
    }
}

fn tick_hint(
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Hinted, &mut Sprite)>,
) {
    for (entity, mut hint, mut sprite) in &mut q {
        hint.t += time.delta_secs() * settings.anim_speed;
        if hint.t >= Hinted::DURATION {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<Hinted>();
        } else {
            let pulse = 0.5 + 0.5 * (hint.t * 12.0).sin();
            sprite.color = Color::WHITE.mix(&crate::theme::GOLD, 0.55 * pulse);
        }
    }
}

fn move_cascading(
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
    mut q: Query<(&mut Transform, &mut Cascading)>,
) {
    let dt = time.delta_secs().min(1.0 / 30.0) * settings.anim_speed;
    let floor = -VIRTUAL_H / 2.0 + crate::layout::CARD_H / 2.0 - 40.0;
    for (mut transform, mut c) in &mut q {
        c.vel.y -= 2800.0 * dt;
        transform.translation.x += c.vel.x * dt;
        transform.translation.y += c.vel.y * dt;
        transform.rotation *= Quat::from_rotation_z(c.vel.x * 0.0006 * dt * 60.0);
        if transform.translation.y < floor && c.vel.y < 0.0 {
            transform.translation.y = floor;
            c.vel.y = -c.vel.y * 0.82;
        }
    }
}
