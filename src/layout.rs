use bevy::prelude::*;

use crate::cards::CARD_PX;

pub const SCALE: f32 = 3.5;
pub const CARD_W: f32 = (CARD_PX.x - 1) as f32 * SCALE;
pub const CARD_H: f32 = (CARD_PX.y - 1) as f32 * SCALE;

pub const VIRTUAL_W: f32 = 1280.0;
pub const VIRTUAL_H: f32 = 800.0;

pub const COL_SPACING: f32 = 154.0;
/// Center y of the top row (stock, waste, foundations).
pub const TOP_Y: f32 = 240.0;
/// Center y of the first card in each tableau column.
pub const TABLEAU_TOP_Y: f32 = 30.0;
/// Lowest allowed center y for a fanned tableau card.
pub const BOTTOM_LIMIT: f32 = -VIRTUAL_H / 2.0 + CARD_H / 2.0 + 10.0;

pub const FAN_REVEALED: f32 = 36.0;
pub const FAN_HIDDEN: f32 = 14.0;

pub fn col_x(i: usize) -> f32 {
    (i as f32 - 3.0) * COL_SPACING
}

pub fn stock_pos() -> Vec2 {
    Vec2::new(col_x(0), TOP_Y)
}

pub fn waste_pos() -> Vec2 {
    Vec2::new(col_x(1), TOP_Y)
}

pub fn foundation_pos(i: usize) -> Vec2 {
    Vec2::new(col_x(3 + i), TOP_Y)
}

/// Fan offsets for a tableau pile, compressed so long piles stay on screen.
/// `revealed[i]` is whether card i is face up. Returns center position of card `idx`.
pub fn tableau_card_pos(col: usize, idx: usize, revealed: &[bool]) -> Vec2 {
    let x = col_x(col);
    let mut raw = 0.0;
    let mut offsets = Vec::with_capacity(revealed.len());
    for i in 0..revealed.len() {
        offsets.push(raw);
        raw += if revealed[i] {
            FAN_REVEALED
        } else {
            FAN_HIDDEN
        };
    }
    let last = offsets.last().copied().unwrap_or(0.0);
    let avail = TABLEAU_TOP_Y - BOTTOM_LIMIT;
    let factor = if last > avail { avail / last } else { 1.0 };
    Vec2::new(x, TABLEAU_TOP_Y - offsets[idx] * factor)
}

pub fn foundation_rect(i: usize) -> Rect {
    let c = foundation_pos(i);
    Rect::from_center_size(c, Vec2::new(CARD_W + 24.0, CARD_H + 24.0))
}

/// Whole column strip below the top row; empty piles included.
pub fn tableau_rect(col: usize) -> Rect {
    let x = col_x(col);
    let top = TABLEAU_TOP_Y + CARD_H / 2.0 + 12.0;
    let bottom = -VIRTUAL_H / 2.0;
    Rect::from_corners(
        Vec2::new(x - CARD_W / 2.0 - 12.0, bottom),
        Vec2::new(x + CARD_W / 2.0 + 12.0, top),
    )
}
