use bevy::prelude::*;

use crate::engine::card::{Card, Rank, Suit};

pub const ATLAS_COLS: u32 = 13;
pub const ATLAS_ROWS: u32 = 11;
pub const CARD_PX: UVec2 = UVec2::new(40, 56);

/// Red card back, third card in the red backs row.
pub const BACK_INDEX: usize = 6 * ATLAS_COLS as usize + 2;
/// Slot/shadow silhouette index; this atlas has no separate blank card.
pub const WHITE_BASE_INDEX: usize = BACK_INDEX;

/// Child entity holding the face art overlay of a card.
#[derive(Component)]
pub struct FaceArt;

#[derive(Resource)]
pub struct CardAssets {
    pub faces_img: Handle<Image>,
    pub faces_layout: Handle<TextureAtlasLayout>,
    pub backs_img: Handle<Image>,
    pub backs_layout: Handle<TextureAtlasLayout>,
    pub font: Handle<Font>,
}

/// Sheet rows: hearts, diamonds, clubs, spades.
/// Sheet columns: A,2,3,4,5,6,7,8,9,10,J,Q,K.
pub fn face_index(card: &Card) -> usize {
    let row = match card.suit() {
        Suit::Heart => 0,
        Suit::Diamonds => 1,
        Suit::Club => 2,
        Suit::Spades => 3,
    };
    let col = card.rank_u8() as usize;
    row * 13 + col
}

/// Inverse of Card::id() (suit * 13 + rank).
pub fn card_from_id(id: u8) -> Card {
    let suit = match id / 13 {
        0 => Suit::Diamonds,
        1 => Suit::Spades,
        2 => Suit::Heart,
        _ => Suit::Club,
    };
    let rank = match id % 13 {
        0 => Rank::Ace,
        1 => Rank::Two,
        2 => Rank::Three,
        3 => Rank::Four,
        4 => Rank::Five,
        5 => Rank::Six,
        6 => Rank::Seven,
        7 => Rank::Eight,
        8 => Rank::Nine,
        9 => Rank::Ten,
        10 => Rank::Jack,
        11 => Rank::Queen,
        _ => Rank::King,
    };
    Card {
        suit,
        rank,
        revealed: false,
    }
}

/// Full-card sprites live on the base entity. The face-art child is kept hidden
/// so the rest of the card entity plumbing stays unchanged.
pub fn set_face(
    base: &mut Sprite,
    _art: &mut Sprite,
    art_vis: &mut Visibility,
    assets: &CardAssets,
    card: &Card,
    face_up: bool,
) {
    let base_index = if face_up {
        face_index(card)
    } else {
        BACK_INDEX
    };
    if !base
        .texture_atlas
        .as_ref()
        .is_some_and(|a| a.index == base_index)
    {
        base.image = assets.faces_img.clone();
        base.texture_atlas = Some(TextureAtlas {
            layout: assets.faces_layout.clone(),
            index: base_index,
        });
    }
    *art_vis = Visibility::Hidden;
}

/// Insert CardAssets during plugin build so it exists before the initial
/// state transition (OnEnter(MainMenu) runs before Startup).
pub fn insert_assets(app: &mut App) {
    let world = app.world_mut();
    let server = world.resource::<AssetServer>().clone();
    let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
    let cards_layout = layouts.add(TextureAtlasLayout::from_grid(
        CARD_PX, ATLAS_COLS, ATLAS_ROWS, None, None,
    ));
    let cards_img: Handle<Image> = server.load("full_cards.png");
    world.insert_resource(CardAssets {
        faces_img: cards_img.clone(),
        faces_layout: cards_layout.clone(),
        backs_img: cards_img,
        backs_layout: cards_layout,
        font: server.load(crate::theme::FONT),
    });
}
