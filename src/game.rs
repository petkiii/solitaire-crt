use bevy::prelude::*;
use rand::Rng;

use crate::animation::{Cascading, Flip, Hinted, Juice, Moveable};
use crate::cards::{CardAssets, set_face};
use crate::deal::new_deal;
use crate::engine::card::Card;
use crate::engine::errors::MoveError;
use crate::engine::game_state::{GameState, Move, can_place_on_foundation};
use crate::engine::{AutoMove, Engine};
use crate::input::DragState;
use crate::layout;

#[derive(Message)]
pub struct NewGameMsg;

#[derive(Message)]
pub struct UndoMsg;

#[derive(Message)]
pub struct HintMsg;

#[derive(Message)]
pub struct AutoCompleteMsg;

#[derive(Resource)]
pub struct Session {
    pub engine: Engine,
    pub score: i32,
    score_hist: Vec<i32>,
    pub elapsed: f32,
    pub running: bool,
    pub won: bool,
    pub auto: bool,
}

impl Session {
    fn new() -> Self {
        Self {
            engine: Engine::from_state(new_deal()),
            score: 0,
            score_hist: Vec::new(),
            elapsed: 0.0,
            running: false,
            won: false,
            auto: false,
        }
    }

    pub fn play(&mut self, mv: Move) -> Result<(), MoveError> {
        let hidden_before = count_hidden(self.engine.state());
        self.engine.apply_move(mv)?;
        let mut delta = match mv {
            Move::WasteToTableau { .. } => 5,
            Move::WasteToFoundation { .. } => 10,
            Move::TableauToFoundation { .. } => 10,
            Move::FoundationToTableau { .. } => -15,
            Move::Recycle => -100,
            _ => 0,
        };
        if count_hidden(self.engine.state()) < hidden_before {
            delta += 5;
        }
        let applied = (self.score + delta).max(0) - self.score;
        self.score += applied;
        self.score_hist.push(applied);
        self.running = true;
        if self.engine.state().is_won() {
            self.won = true;
            self.running = false;
        }
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), MoveError> {
        self.engine.apply_undo()?;
        if let Some(d) = self.score_hist.pop() {
            self.score -= d;
        }
        Ok(())
    }

    /// Resolve "send this card somewhere sensible" (double-click) to a Move
    /// via the engine's pure resolver; the caller applies it through `play`
    /// so scoring stays in one place.
    pub fn auto_move_for(&self, loc: CardLoc) -> Option<Move> {
        let state = self.engine.state();
        let auto = match loc {
            CardLoc::Waste(i) => (i + 1 == state.waste_len()).then_some(AutoMove::WasteCard)?,
            CardLoc::Tableau(col, idx) => AutoMove::TableauCard {
                tableau_idx: col,
                card_idx: idx,
            },
            CardLoc::Foundation(f, i) => (i + 1 == state.foundation_len(f))
                .then_some(AutoMove::FoundationCard { foundation_idx: f })?,
        };
        self.engine.resolve_auto_move(auto).ok()
    }
}

fn find_foundation(state: &GameState, card: &Card) -> Option<usize> {
    state
        .foundations_iter()
        .position(|f| can_place_on_foundation(card, f.last()))
}

fn count_hidden(state: &GameState) -> usize {
    state
        .tableaus_iter()
        .flat_map(|t| t.iter())
        .filter(|c| !c.revealed())
        .count()
}

/// Auto-complete unlocks once every tableau card is face-up: from there the
/// game is always winnable (draw-1, unlimited recycles), stock/waste included.
pub fn auto_complete_available(state: &GameState) -> bool {
    !state.is_won()
        && state
            .tableaus_iter()
            .flat_map(|t| t.iter())
            .all(|c| c.revealed())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CardLoc {
    Waste(usize),
    Foundation(usize, usize),
    Tableau(usize, usize),
}

/// Where each card id currently lives, rebuilt on demand.
/// Stock cards are absent (`None`): the engine only exposes stock length, and
/// face-down stock cards all render identically on the stock slot anyway.
pub fn locate_cards(state: &GameState) -> [Option<(CardLoc, Card)>; 52] {
    let mut locs = [None; 52];
    for (i, &c) in state.waste().iter().enumerate() {
        locs[c.id() as usize] = Some((CardLoc::Waste(i), c));
    }
    for f in 0..4 {
        for (i, &c) in state.foundation(f).iter().enumerate() {
            locs[c.id() as usize] = Some((CardLoc::Foundation(f, i), c));
        }
    }
    for (col, pile) in state.tableaus_iter().enumerate() {
        for (i, &c) in pile.iter().enumerate() {
            locs[c.id() as usize] = Some((CardLoc::Tableau(col, i), c));
        }
    }
    locs
}

#[derive(Component)]
pub struct CardEntity {
    pub id: u8,
    pub face_up: bool,
    pub shadow: Entity,
    pub in_stock: bool,
    /// Top of its pile — the only cards that cast idle shadows (overlapping
    /// shadows in fanned piles stack into dark blocks otherwise).
    pub exposed: bool,
    /// Face-art overlay child (see cards::set_face).
    pub art: Entity,
}

#[derive(Component)]
struct CardShadow;

/// id -> entity for the 52 card sprites.
#[derive(Resource)]
pub struct CardIndex(pub [Option<Entity>; 52]);

impl Default for CardIndex {
    fn default() -> Self {
        Self([None; 52])
    }
}

#[derive(Component)]
pub struct StockMarker;

#[derive(Component)]
pub struct PileMarker;

#[derive(Resource)]
struct WinCascade {
    timer: Timer,
    remaining: Vec<Entity>,
}

pub fn plugin(app: &mut App) {
    app.add_message::<NewGameMsg>()
        .add_message::<UndoMsg>()
        .add_message::<HintMsg>()
        .add_message::<AutoCompleteMsg>()
        .init_resource::<CardIndex>()
        .insert_resource(Session::new());
    crate::cards::insert_assets(app);
    app.add_systems(Startup, setup_table).add_systems(
        Update,
        (
            handle_new_game,
            handle_undo,
            handle_hint,
            handle_auto_complete,
            tick_auto_complete,
            tick_timer,
            sync_board,
            sync_card_shadows,
            start_win_cascade,
            tick_win_cascade,
        )
            .chain(),
    );
}

fn setup_table(mut commands: Commands, assets: Res<CardAssets>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::AutoMin {
                min_width: layout::VIRTUAL_W,
                min_height: layout::VIRTUAL_H,
            },
            ..OrthographicProjection::default_2d()
        }),
        crate::post::CrtPost::default(),
    ));

    // Pile slots: one card-shaped dark well, no outer rim. Keep the 1.04
    // footprint so the empty-stock click target stays generous.
    let slot = |pos: Vec2, marker: bool, commands: &mut Commands| {
        let slot_sprite = |color: Color| Sprite {
            image: assets.backs_img.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: assets.backs_layout.clone(),
                index: crate::cards::WHITE_BASE_INDEX,
            }),
            color,
            ..default()
        };
        let mut e = commands.spawn((
            slot_sprite(crate::theme::PILE_INNER),
            Transform::from_translation(pos.extend(0.1))
                .with_scale(Vec3::splat(layout::SCALE * 1.04)),
            PileMarker,
            Pickable::default(),
        ));
        if marker {
            e.insert(StockMarker);
        }
    };

    slot(layout::stock_pos(), true, &mut commands);
    slot(layout::waste_pos(), false, &mut commands);
    for i in 0..4 {
        slot(layout::foundation_pos(i), false, &mut commands);
    }
    for col in 0..7 {
        slot(
            Vec2::new(layout::col_x(col), layout::TABLEAU_TOP_Y),
            false,
            &mut commands,
        );
    }
}

fn handle_new_game(
    mut msgs: MessageReader<NewGameMsg>,
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut index: ResMut<CardIndex>,
    mut drag: ResMut<DragState>,
    assets: Res<CardAssets>,
    settings: Res<crate::settings::Settings>,
    existing: Query<Entity, With<CardEntity>>,
) {
    if msgs.read().next().is_none() {
        return;
    }
    for e in &existing {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<WinCascade>();
    drag.active = None; // stale drag would point at despawned entities
    *session = Session::new();
    *index = CardIndex::default();

    let state = session.engine.state();
    let locs = locate_cards(state);
    let stock_pos = layout::stock_pos();

    for id in 0..52u8 {
        let card = crate::cards::card_from_id(id);
        let mut base = Sprite::default();
        let mut art = Sprite::default();
        let mut art_vis = Visibility::Hidden;
        set_face(&mut base, &mut art, &mut art_vis, &assets, &card, false);

        let delay = match locs[id as usize] {
            Some((CardLoc::Tableau(col, row), _)) => {
                (0.35 + (row as f32 * 7.0 + col as f32) * 0.045) / settings.anim_speed
            }
            _ => 0.0,
        };
        let mut moveable = Moveable::at(stock_pos, 1.0);
        moveable.delay = delay;

        let shadow_entity = commands
            .spawn((
                Sprite {
                    image: assets.backs_img.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: assets.backs_layout.clone(),
                        index: crate::cards::WHITE_BASE_INDEX,
                    }),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.30),
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, -15.0 / layout::SCALE, -0.25))
                    .with_scale(Vec3::splat(0.98)),
                Visibility::Hidden,
                CardShadow,
                Pickable::IGNORE,
            ))
            .id();
        let art_entity = commands
            .spawn((
                art,
                art_vis,
                crate::cards::FaceArt,
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.01)),
                Pickable::IGNORE,
            ))
            .id();
        let entity = commands
            .spawn((
                base,
                Transform::from_translation(stock_pos.extend(1.0))
                    .with_scale(Vec3::splat(layout::SCALE)),
                CardEntity {
                    id,
                    face_up: false,
                    shadow: shadow_entity,
                    in_stock: true,
                    exposed: false,
                    art: art_entity,
                },
                moveable,
                Pickable::default(),
            ))
            .add_child(shadow_entity)
            .add_child(art_entity)
            .id();
        index.0[id as usize] = Some(entity);
    }
}

fn handle_undo(mut msgs: MessageReader<UndoMsg>, mut session: ResMut<Session>) {
    for _ in msgs.read() {
        if session.won {
            continue;
        }
        session.auto = false;
        _ = session.undo();
    }
}

fn handle_hint(
    mut msgs: MessageReader<HintMsg>,
    mut commands: Commands,
    session: Res<Session>,
    index: Res<CardIndex>,
    markers: Query<Entity, With<StockMarker>>,
) {
    if msgs.read().next().is_none() {
        return;
    }
    let state = session.engine.state();
    let Some(mv) = state.hint_move() else { return };

    let top_id = |cards: &[Card]| cards.last().map(|c| c.id());
    let (src, dst) = match mv {
        Move::Draw | Move::Recycle => (None, None),
        Move::WasteToFoundation { foundation_idx } => (
            top_id(state.waste()),
            top_id(state.foundation(foundation_idx)),
        ),
        Move::WasteToTableau { tableau_idx } => {
            (top_id(state.waste()), top_id(state.tableau(tableau_idx)))
        }
        Move::TableauToFoundation {
            tableau_idx,
            foundation_idx,
        } => (
            top_id(state.tableau(tableau_idx)),
            top_id(state.foundation(foundation_idx)),
        ),
        Move::FoundationToTableau {
            foundation_idx,
            tableau_idx,
        } => (
            top_id(state.foundation(foundation_idx)),
            top_id(state.tableau(tableau_idx)),
        ),
        Move::TableauToTableau {
            src_tableau_idx,
            card_idx,
            dst_tableau_idx,
        } => (
            state.tableau(src_tableau_idx).get(card_idx).map(|c| c.id()),
            top_id(state.tableau(dst_tableau_idx)),
        ),
    };

    if matches!(mv, Move::Draw | Move::Recycle) {
        for e in &markers {
            commands.entity(e).insert(Juice::new(0.3));
        }
        return;
    }
    for id in [src, dst].into_iter().flatten() {
        if let Some(entity) = index.0[id as usize] {
            commands
                .entity(entity)
                .insert((Hinted { t: 0.0 }, Juice::new(0.25)));
        }
    }
}

fn handle_auto_complete(mut msgs: MessageReader<AutoCompleteMsg>, mut session: ResMut<Session>) {
    if msgs.read().next().is_some() && auto_complete_available(session.engine.state()) {
        session.auto = true;
    }
}

fn tick_auto_complete(
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    index: Res<CardIndex>,
    mut cooldown: Local<f32>,
    mut fruitless: Local<u32>,
) {
    if !session.auto {
        *fruitless = 0;
        return;
    }
    *cooldown -= time.delta_secs();
    if *cooldown > 0.0 {
        return;
    }

    let state = session.engine.state();
    let stock_len = state.stock_len();
    let waste_len = state.waste_len();

    // Prefer a foundation play: tableau tops first, then the waste top.
    let mut found = None;
    for (tableau_idx, pile) in state.tableaus_iter().enumerate() {
        if let Some(card) = pile.last()
            && let Some(foundation_idx) = find_foundation(state, card)
        {
            found = Some((
                Move::TableauToFoundation {
                    tableau_idx,
                    foundation_idx,
                },
                card.id(),
            ));
            break;
        }
    }
    if found.is_none()
        && let Some(card) = state.waste().last()
        && let Some(foundation_idx) = find_foundation(state, card)
    {
        found = Some((Move::WasteToFoundation { foundation_idx }, card.id()));
    }

    match found {
        Some((mv, id)) => {
            *fruitless = 0;
            *cooldown = 0.13 / settings.anim_speed;
            _ = session.play(mv);
            if let Some(entity) = index.0[id as usize] {
                commands.entity(entity).insert(Juice::new(0.3));
            }
        }
        None => {
            // Cycle the stock until the next playable card surfaces. Worst
            // legitimate barren streak is draining the stock, recycling, and
            // drawing through once more (~2x the cycle); anything past that
            // means the game can't finish (only reachable via debug reveal;
            // real all-revealed games are always winnable).
            let cycle = stock_len + waste_len;
            if cycle == 0 || *fruitless as usize > 2 * cycle + 4 {
                session.auto = false;
                *fruitless = 0;
                return;
            }
            *fruitless += 1;
            *cooldown = 0.045 / settings.anim_speed;
            let mv = if stock_len > 0 {
                Move::Draw
            } else {
                Move::Recycle
            };
            _ = session.play(mv);
        }
    }
}

fn tick_timer(time: Res<Time>, state: Res<State<crate::AppState>>, mut session: ResMut<Session>) {
    if *state.get() == crate::AppState::Playing && session.running && !session.won {
        session.elapsed += time.delta_secs();
    }
}

/// Every frame: point every card sprite at where the engine says it lives.
/// Skips dragged and cascading cards; starts flips when a settled card's
/// shown face disagrees with the engine.
fn sync_board(
    session: Res<Session>,
    drag: Res<DragState>,
    assets: Res<CardAssets>,
    mut commands: Commands,
    index: Res<CardIndex>,
    mut q: Query<
        (
            &mut CardEntity,
            &mut Moveable,
            &mut Sprite,
            &Transform,
            Option<&Flip>,
            Option<&Hinted>,
        ),
        Without<Cascading>,
    >,
    mut art_q: Query<
        (&mut Sprite, &mut Visibility),
        (With<crate::cards::FaceArt>, Without<CardEntity>),
    >,
) {
    let state = session.engine.state();
    let locs = locate_cards(state);

    // Tableau reveal maps for fan spacing.
    let revealed: Vec<Vec<bool>> = state
        .tableaus_iter()
        .map(|t| t.iter().map(|c| c.revealed()).collect())
        .collect();

    // Cards the engine can't see individually (stock) stack on the stock pos.
    // Engine hides their order, so id order stands in for stack order — it
    // just needs to be stable so render/picking agree and offsets don't jump.
    let stock_pos = layout::stock_pos();
    let waste_pos = layout::waste_pos();
    let stock_ids: Vec<u8> = (0..52u8)
        .filter(|&id| locs[id as usize].is_none())
        .collect();

    for id in 0..52u8 {
        let Some(entity) = index.0[id as usize] else {
            continue;
        };
        if drag.holds(entity) {
            continue;
        }
        let Ok((mut ce, mut m, mut sprite, transform, flip, hinted)) = q.get_mut(entity) else {
            continue;
        };

        let stock_k = stock_ids.iter().position(|&s| s == id);
        let (pos, z, face_up, card, exposed) = match locs[id as usize] {
            Some((CardLoc::Waste(i), c)) => (
                waste_pos,
                1.0 + i as f32 * 0.05,
                true,
                c,
                i + 1 == state.waste_len(),
            ),
            Some((CardLoc::Foundation(f, i), c)) => (
                layout::foundation_pos(f),
                1.0 + i as f32 * 0.05,
                true,
                c,
                i + 1 == state.foundation_len(f),
            ),
            Some((CardLoc::Tableau(col, i), c)) => (
                layout::tableau_card_pos(col, i, &revealed[col]),
                1.0 + i as f32 * 0.5,
                c.revealed(),
                c,
                i + 1 == revealed[col].len(),
            ),
            None => {
                // Stock card. The bottom card sits flush on the slot and the
                // pile builds up-left from it, so the slot's bottom/right rim
                // stays visible beside the stack.
                let k = stock_k.unwrap_or(0);
                let step = (k as f32).min(8.0);
                (
                    stock_pos + Vec2::new(-0.9, 1.0) * step,
                    1.5 + k as f32 * 0.005,
                    false,
                    crate::cards::card_from_id(id),
                    false,
                )
            }
        };

        ce.in_stock = stock_k.is_some();
        ce.exposed = exposed;
        m.target = pos;
        m.z = z;

        // Deeper stock cards get a darker body so the slivers peeking out
        // beneath the top card read as stacked edges (white-on-white hides
        // them otherwise). Skip while the hint pulse owns the tint.
        let tint = match stock_k {
            Some(k) => {
                let step = (k as f32).min(8.0);
                let top_step = ((stock_ids.len() - 1) as f32).min(8.0);
                let g = 1.0 - 0.05 * (top_step - step);
                Color::srgb(g, g, g)
            }
            None => Color::WHITE,
        };
        if hinted.is_none() && sprite.color != tint {
            sprite.color = tint;
        }

        if ce.face_up != face_up && flip.is_none() && m.is_settled(transform) {
            ce.face_up = face_up;
            commands.entity(entity).insert(Flip {
                t: 0.0,
                card,
                face_up,
                swapped: false,
            });
        } else if ce.face_up == face_up
            && flip.is_none()
            && let Ok((mut art, mut art_vis)) = art_q.get_mut(ce.art)
        {
            // Keep the face art in sync (e.g. entity respawn or undo mid-flip).
            set_face(&mut sprite, &mut art, &mut art_vis, &assets, &card, face_up);
        }
    }
}

fn sync_card_shadows(
    cards: Query<(&CardEntity, &Moveable, &Transform)>,
    mut shadows: Query<(&mut Transform, &mut Visibility), (With<CardShadow>, Without<CardEntity>)>,
) {
    for (ce, m, card_tf) in &cards {
        let Ok((mut shadow_tf, mut visibility)) = shadows.get_mut(ce.shadow) else {
            continue;
        };
        // Idle shadows only under pile tops (CardEntity.exposed, kept fresh
        // by sync_board right before this system). Dragged cards always cast
        // one: sync_board skips held cards so `exposed` may be stale for
        // them, but `dragging` covers the whole held stack.
        let want = if m.dragging || (ce.exposed && !ce.in_stock && m.delay <= 0.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }

        let height = if m.dragging { 0.35 } else { 0.1 };
        let normalized_x = card_tf.translation.x / (layout::VIRTUAL_W * 0.5);
        // Shadow is a child of the card sprite, so keep pixel offsets stable
        // when the parent card scale changes.
        let shadow_offset = Vec2::new(
            -normalized_x * 1.5 * height * crate::animation::PX_PER_UNIT,
            -1.5 * height * crate::animation::PX_PER_UNIT,
        ) / layout::SCALE;
        shadow_tf.translation = shadow_offset.extend(-0.25);
        shadow_tf.scale = Vec3::splat(1.0 - 0.2 * height);
    }
}

fn start_win_cascade(
    mut commands: Commands,
    session: Res<Session>,
    index: Res<CardIndex>,
    cascade: Option<Res<WinCascade>>,
) {
    if !session.won || cascade.is_some() {
        return;
    }
    let state = session.engine.state();
    // Round-robin foundations, top card first.
    let mut remaining = Vec::with_capacity(52);
    for depth in 0..13 {
        for f in 0..4 {
            let pile = state.foundation(f);
            if let Some(card) = pile.get(pile.len().wrapping_sub(1 + depth))
                && let Some(e) = index.0[card.id() as usize]
            {
                remaining.push(e);
            }
        }
    }
    if remaining.is_empty() {
        // Won without full foundations only happens via the dev win preview.
        remaining.extend(index.0.iter().flatten());
    }
    // tick_win_cascade pops from the back; launch top cards first.
    remaining.reverse();
    commands.insert_resource(WinCascade {
        timer: Timer::from_seconds(0.09, TimerMode::Repeating),
        remaining,
    });
}

fn tick_win_cascade(time: Res<Time>, mut commands: Commands, cascade: Option<ResMut<WinCascade>>) {
    let Some(mut cascade) = cascade else { return };
    cascade.timer.tick(time.delta());
    for _ in 0..cascade.timer.times_finished_this_tick() {
        let Some(entity) = cascade.remaining.pop() else {
            return;
        };
        let mut rng = rand::rng();
        let vx: f32 =
            rng.random_range(150.0..600.0) * if rng.random::<bool>() { 1.0 } else { -1.0 };
        let vy: f32 = rng.random_range(200.0..700.0);
        commands.entity(entity).insert(Cascading {
            vel: Vec2::new(vx, vy),
        });
    }
}
