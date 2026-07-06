use bevy::prelude::*;

use crate::animation::{Cascading, Juice, Moveable};
use crate::engine::game_state::{Move, can_grab_from_tableau};
use crate::game::{CardEntity, CardIndex, CardLoc, Session, StockMarker, locate_cards};
use crate::layout;

/// Cards currently being dragged, in stack order (grabbed card first).
#[derive(Resource, Default)]
pub struct DragState {
    pub active: Option<DragInfo>,
}

pub struct DragInfo {
    pub source: CardLoc,
    pub entities: Vec<Entity>,
    /// Cursor offset from the grabbed card's center at drag start.
    pub grab_offset: Vec2,
}

impl DragState {
    pub fn holds(&self, entity: Entity) -> bool {
        self.active
            .as_ref()
            .is_some_and(|d| d.entities.contains(&entity))
    }
}

#[derive(Resource, Default)]
struct LastClick {
    entity: Option<Entity>,
    at: f32,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<DragState>()
        .init_resource::<LastClick>()
        .add_systems(Update, keyboard)
        .add_observer(on_over)
        .add_observer(on_out)
        .add_observer(on_drag_start)
        .add_observer(on_drag)
        .add_observer(on_drag_end)
        .add_observer(on_click);
}

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<crate::AppState>>,
    mut session: ResMut<Session>,
    mut new_game: MessageWriter<crate::game::NewGameMsg>,
    mut undo: MessageWriter<crate::game::UndoMsg>,
    mut hint: MessageWriter<crate::game::HintMsg>,
    mut auto: MessageWriter<crate::game::AutoCompleteMsg>,
) {
    if *state.get() != crate::AppState::Playing {
        return;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        new_game.write(crate::game::NewGameMsg);
    }
    if session.won || session.auto {
        return;
    }
    if keys.just_pressed(KeyCode::KeyU) {
        undo.write(crate::game::UndoMsg);
    }
    if keys.just_pressed(KeyCode::KeyD) {
        let state = session.engine.state();
        let mv = if state.stock_len() > 0 {
            Move::Draw
        } else {
            Move::Recycle
        };
        _ = session.play(mv);
    }
    if keys.just_pressed(KeyCode::KeyH) {
        hint.write(crate::game::HintMsg);
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyA) {
        auto.write(crate::game::AutoCompleteMsg);
    }
    // Dev-only: preview the win celebration.
    #[cfg(debug_assertions)]
    if keys.just_pressed(KeyCode::KeyT) {
        session.won = true;
    }
    // Dev-only: reveal all tableau cards (tests early auto-complete).
    #[cfg(debug_assertions)]
    if keys.just_pressed(KeyCode::KeyR) {
        session.engine.debug_reveal_all();
    }
}

fn cursor_world(pointer_pos: Vec2, camera_q: &Query<(&Camera, &GlobalTransform)>) -> Option<Vec2> {
    let (camera, cam_transform) = camera_q.single().ok()?;
    camera.viewport_to_world_2d(cam_transform, pointer_pos).ok()
}

fn on_over(
    over: On<Pointer<Over>>,
    session: Res<Session>,
    drag: Res<DragState>,
    state: Res<State<crate::AppState>>,
    mut q: Query<(&CardEntity, &mut Moveable)>,
) {
    if drag.active.is_some() || session.won || *state.get() != crate::AppState::Playing {
        return;
    }
    let Ok((ce, mut m)) = q.get_mut(over.event_target()) else {
        return;
    };
    let state_ref = session.engine.state();
    let in_stock = locate_cards(state_ref)[ce.id as usize].is_none();
    // Lift cards that could be picked up, plus the stock top (clickable).
    if in_stock || grabbable(&session, ce.id).is_some() {
        m.hover = true;
    }
}

fn on_out(out: On<Pointer<Out>>, mut q: Query<&mut Moveable, With<CardEntity>>) {
    if let Ok(mut m) = q.get_mut(out.event_target()) {
        m.hover = false;
    }
}

/// If this card may start a drag, return its location and the ids of the
/// substack it carries (itself + everything fanned on top of it).
fn grabbable(session: &Session, id: u8) -> Option<(CardLoc, Vec<u8>)> {
    let state = session.engine.state();
    let locs = locate_cards(state);
    let (loc, _) = locs[id as usize]?;
    match loc {
        CardLoc::Waste(i) => (i + 1 == state.waste_len()).then(|| (loc, vec![id])),
        CardLoc::Foundation(f, i) => (i + 1 == state.foundation_len(f)).then(|| (loc, vec![id])),
        CardLoc::Tableau(col, i) => {
            let pile = state.tableau(col);
            let cards = pile.get(i..)?;
            can_grab_from_tableau(cards).then(|| (loc, cards.iter().map(|c| c.id()).collect()))
        }
    }
}

fn on_drag_start(
    start: On<Pointer<DragStart>>,
    session: Res<Session>,
    mut drag: ResMut<DragState>,
    index: Res<CardIndex>,
    state: Res<State<crate::AppState>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut q: Query<(&CardEntity, &mut Moveable, &Transform)>,
) {
    if drag.active.is_some()
        || session.won
        || session.auto
        || *state.get() != crate::AppState::Playing
    {
        return;
    }
    let entity = start.event_target();
    let Ok((ce, _, transform)) = q.get_mut(entity) else {
        return;
    };
    let Some((source, ids)) = grabbable(&session, ce.id) else {
        return;
    };
    let Some(cursor) = cursor_world(start.pointer_location.position, &camera_q) else {
        return;
    };

    let grab_offset = transform.translation.truncate() - cursor;
    let entities: Vec<Entity> = ids.iter().filter_map(|&id| index.0[id as usize]).collect();

    for (i, &e) in entities.iter().enumerate() {
        if let Ok((_, mut m, _)) = q.get_mut(e) {
            m.dragging = true;
            m.hover = false;
            m.z = 600.0 + i as f32;
        }
    }
    drag.active = Some(DragInfo {
        source,
        entities,
        grab_offset,
    });
}

fn on_drag(
    ev: On<Pointer<Drag>>,
    drag: Res<DragState>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut q: Query<&mut Moveable, With<CardEntity>>,
) {
    let Some(info) = &drag.active else { return };
    let Some(cursor) = cursor_world(ev.pointer_location.position, &camera_q) else {
        return;
    };
    let base = cursor + info.grab_offset;
    for (i, &e) in info.entities.iter().enumerate() {
        if let Ok(mut m) = q.get_mut(e) {
            m.target = base - Vec2::new(0.0, i as f32 * layout::FAN_REVEALED);
            m.z = 600.0 + i as f32;
        }
    }
}

fn on_drag_end(
    ev: On<Pointer<DragEnd>>,
    mut drag: ResMut<DragState>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut q: Query<&mut Moveable, With<CardEntity>>,
) {
    let Some(info) = drag.active.take() else {
        return;
    };
    for &e in &info.entities {
        if let Ok(mut m) = q.get_mut(e) {
            m.dragging = false;
        }
    }

    let Some(cursor) = cursor_world(ev.pointer_location.position, &camera_q) else {
        return;
    };
    let drop = cursor + info.grab_offset;

    if let Some(mv) = drop_move(&session, info.source, drop)
        && session.play(mv).is_ok()
    {
        return;
    }
    // Invalid: shake, spring return happens via sync.
    for &e in &info.entities {
        commands.entity(e).insert(Juice::new(0.25));
    }
}

/// Map a drop position + source pile to a concrete Move, if the drop landed
/// on a plausible zone. Legality is still the engine's call.
fn drop_move(session: &Session, source: CardLoc, drop: Vec2) -> Option<Move> {
    for f in 0..4 {
        if layout::foundation_rect(f).contains(drop) {
            return match source {
                CardLoc::Waste(_) => Some(Move::WasteToFoundation { foundation_idx: f }),
                CardLoc::Tableau(col, i) if i + 1 == session.engine.state().tableau(col).len() => {
                    Some(Move::TableauToFoundation {
                        tableau_idx: col,
                        foundation_idx: f,
                    })
                }
                _ => None,
            };
        }
    }
    for col in 0..7 {
        if layout::tableau_rect(col).contains(drop) {
            return match source {
                CardLoc::Waste(_) => Some(Move::WasteToTableau { tableau_idx: col }),
                CardLoc::Foundation(f, _) => Some(Move::FoundationToTableau {
                    foundation_idx: f,
                    tableau_idx: col,
                }),
                CardLoc::Tableau(src, i) if src != col => Some(Move::TableauToTableau {
                    src_tableau_idx: src,
                    card_idx: i,
                    dst_tableau_idx: col,
                }),
                _ => None,
            };
        }
    }
    None
}

fn on_click(
    click: On<Pointer<Click>>,
    time: Res<Time>,
    mut last: ResMut<LastClick>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    state: Res<State<crate::AppState>>,
    cards: Query<&CardEntity, Without<Cascading>>,
    stock_markers: Query<(), With<StockMarker>>,
) {
    if session.won || session.auto || *state.get() != crate::AppState::Playing {
        return;
    }
    let entity = click.event_target();

    // Empty stock slot: recycle the waste.
    if stock_markers.contains(entity) {
        if session.engine.state().stock_len() == 0 && session.engine.state().waste_len() > 0 {
            _ = session.play(Move::Recycle);
        }
        return;
    }

    let Ok(ce) = cards.get(entity) else { return };
    let state = session.engine.state();
    let locs = locate_cards(state);

    // Stock cards aren't in the location map: click = draw.
    let Some((loc, _)) = locs[ce.id as usize] else {
        _ = session.play(Move::Draw);
        return;
    };

    // Double click sends the card somewhere useful.
    let now = time.elapsed_secs();
    let double = last.entity == Some(entity) && now - last.at < 0.35;
    last.entity = Some(entity);
    last.at = now;
    if !double {
        return;
    }
    last.entity = None;

    if let Some(mv) = session.auto_move_for(loc) {
        if session.play(mv).is_err() {
            commands.entity(entity).insert(Juice::new(0.25));
        }
    } else {
        commands.entity(entity).insert(Juice::new(0.25));
    }
}
