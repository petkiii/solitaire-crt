use crate::engine::game_state::GameState;

/// Single place deals come from. Later this will be replaced by an API that
/// returns a deal for a requested difficulty.
pub fn new_deal() -> GameState {
    GameState::new_random()
}
