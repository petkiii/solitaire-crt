#![allow(dead_code)] // palette constants kept for upcoming card treatments

use bevy::prelude::*;

// Palette direction: near-black blue background, saturated accents.
pub const BG: Color = Color::srgb(0.043, 0.055, 0.102); // near-black blue
pub const PILE_INNER: Color = Color::srgba(0.02, 0.03, 0.06, 0.40);

pub const RED: Color = Color::srgb(0.996, 0.373, 0.333); // FE5F55
pub const BLUE: Color = Color::srgb(0.0, 0.616, 1.0); // 009dff
pub const GOLD: Color = Color::srgb(0.918, 0.753, 0.345); // eac058
pub const ORANGE: Color = Color::srgb(0.992, 0.635, 0.0); // fda200
pub const GREEN: Color = Color::srgb(0.294, 0.761, 0.573); // 4BC292
pub const PURPLE: Color = Color::srgb(0.533, 0.404, 0.647); // 8867a5
pub const DARK: Color = Color::srgb(0.216, 0.259, 0.267); // 374244
pub const L_DARK: Color = Color::srgb(0.310, 0.388, 0.404); // 4f6367

pub const TEXT_LIGHT: Color = Color::srgb(0.92, 0.94, 0.96);
pub const TEXT_DIM: Color = Color::srgba(0.92, 0.94, 0.96, 0.55);

pub const BTN_BG: Color = DARK;
pub const BTN_BG_HOVER: Color = L_DARK;

/// Opaque panel background for menus/dialogs.
pub const PANEL_BG: Color = Color::srgb(0.075, 0.095, 0.13);

/// The one shared backdrop dim behind any open overlay (pause, settings, …).
/// Overlays never bring their own dim, so stacked overlays can't darken the
/// scene twice.
pub const OVERLAY_DIM: Color = Color::srgba(0.0, 0.0, 0.05, 0.7);

pub const FONT: &str = "fonts/m6x11plus.ttf";
