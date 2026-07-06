use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Persisted to `settings.toml` in the working directory (project dir).
pub const SETTINGS_PATH: &str = "settings.toml";

#[derive(Resource, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// Master switch for all screen effects.
    pub effects_enabled: bool,
    /// CRT strength, 0..1: curvature, chroma, scanlines, contrast lift.
    pub crt: f32,
    /// Noise/grain strength, 0..1.
    pub grain: f32,
    /// Edge vignette strength, 0..1.
    pub vignette: f32,
    /// CRT bloom switch; needs CRT > 0 to show.
    pub bloom_enabled: bool,
    /// Animation speed multiplier, 0.5..2.0 (1.0 = designed pace).
    pub anim_speed: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            effects_enabled: true,
            crt: 0.75,
            grain: 0.25,
            vignette: 0.5,
            bloom_enabled: false,
            anim_speed: 1.0,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        match std::fs::read_to_string(SETTINGS_PATH) {
            Ok(text) => match toml::from_str(&text) {
                Ok(s) => s,
                Err(err) => {
                    warn!("settings.toml parse error, using defaults: {err}");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        match toml::to_string_pretty(self) {
            Ok(text) => {
                if let Err(err) = std::fs::write(SETTINGS_PATH, text) {
                    warn!("failed to write {SETTINGS_PATH}: {err}");
                }
            }
            Err(err) => warn!("failed to serialize settings: {err}"),
        }
    }

    /// Effective effect strengths (all zero when master switch is off).
    pub fn fx(&self) -> (f32, f32, f32, f32) {
        if self.effects_enabled {
            let bloom = if self.bloom_enabled { 1.0 } else { 0.0 };
            (self.crt, self.grain, self.vignette, bloom)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    }
}

pub fn plugin(app: &mut App) {
    app.insert_resource(Settings::load())
        .add_systems(Update, save_on_change);
}

fn save_on_change(settings: Res<Settings>, mut skip_first: Local<bool>) {
    if settings.is_changed() {
        if !*skip_first {
            *skip_first = true;
            return; // insertion at startup, nothing to persist yet
        }
        settings.save();
    }
}
