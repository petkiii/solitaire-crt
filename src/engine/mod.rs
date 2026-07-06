#![allow(dead_code)]

pub(crate) mod card;
pub(crate) mod errors;
pub(crate) mod game_state;

use crate::engine::{
    card::Card,
    errors::MoveError,
    game_state::{
        GameState, Move, Undo, can_grab_from_tableau, can_place_on_foundation, can_place_on_tableau,
    },
};

#[derive(Debug)]
pub(crate) enum AutoMove {
    WasteCard,
    TableauCard { tableau_idx: usize, card_idx: usize },
    FoundationCard { foundation_idx: usize },
}

pub(crate) struct Engine {
    state: GameState,
    moves: u32,
    undos: Vec<Undo>,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            state: GameState::new_random(),
            moves: 0,
            undos: Vec::new(),
        }
    }
}

impl Engine {
    pub(crate) fn from_state(state: GameState) -> Self {
        Self {
            state,
            moves: 0,
            undos: Vec::new(),
        }
    }

    fn find_foundation_destination(&self, card: &Card) -> Option<usize> {
        self.state
            .foundations_iter()
            .position(|f| can_place_on_foundation(card, f.last()))
    }

    fn find_tableau_destination(&self, card: &Card, exclude: Option<usize>) -> Option<usize> {
        self.state
            .tableaus_iter()
            .enumerate()
            .find_map(|(idx, tableau)| {
                if exclude == Some(idx) {
                    return None;
                }

                if can_place_on_tableau(card, tableau.last()) {
                    Some(idx)
                } else {
                    None
                }
            })
    }

    pub fn resolve_auto_move(&self, mv: AutoMove) -> Result<Move, MoveError> {
        match mv {
            AutoMove::WasteCard => {
                let Some(card) = self.state.waste_top() else {
                    return Err(MoveError::WasteEmpty);
                };

                if let Some(foundation_idx) = self.find_foundation_destination(card) {
                    return Ok(Move::WasteToFoundation { foundation_idx });
                }

                if let Some(tableau_idx) = self.find_tableau_destination(card, None) {
                    return Ok(Move::WasteToTableau { tableau_idx });
                }

                Err(MoveError::NoValidMoveForWasteCard)
            }
            AutoMove::TableauCard {
                tableau_idx,
                card_idx,
            } => {
                let tableau = self.state.tableau(tableau_idx);

                if card_idx >= tableau.len() {
                    return Err(MoveError::InvalidCardIndex);
                }

                let cards = &tableau[card_idx..];
                if !can_grab_from_tableau(cards) {
                    return Err(MoveError::InvalidCardsToGrab);
                }

                let is_top_card = card_idx == tableau.len() - 1;
                if is_top_card
                    && let Some(foundation_idx) = self.find_foundation_destination(&cards[0])
                {
                    return Ok(Move::TableauToFoundation {
                        tableau_idx,
                        foundation_idx,
                    });
                }

                let Some(dst_tableau_idx) =
                    self.find_tableau_destination(&cards[0], Some(tableau_idx))
                else {
                    return Err(MoveError::NoValidAutoMoveDestinationTableau);
                };

                Ok(Move::TableauToTableau {
                    src_tableau_idx: tableau_idx,
                    card_idx,
                    dst_tableau_idx,
                })
            }
            AutoMove::FoundationCard { foundation_idx } => {
                let foundation = self.state.foundation(foundation_idx);

                let Some(card) = foundation.last() else {
                    return Err(MoveError::FoundationEmpty);
                };

                let Some(tableau_idx) = self.find_tableau_destination(card, None) else {
                    return Err(MoveError::NoValidTableauDestination);
                };

                Ok(Move::FoundationToTableau {
                    foundation_idx,
                    tableau_idx,
                })
            }
        }
    }

    pub fn apply_auto_move(&mut self, mv: AutoMove) -> Result<(), MoveError> {
        let mv = self.resolve_auto_move(mv)?;
        self.apply_move(mv)
    }

    /// Dev-only: reveal every tableau card (auto-complete testing).
    #[cfg(debug_assertions)]
    pub(crate) fn debug_reveal_all(&mut self) {
        self.state.reveal_all_tableau();
    }

    pub(crate) fn apply_move(&mut self, mv: Move) -> Result<(), MoveError> {
        let res = self.state.apply_move(mv);
        match res {
            Ok(undo) => {
                self.moves += 1;
                self.undos.push(undo);

                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub(crate) fn apply_undo(&mut self) -> Result<(), MoveError> {
        let Some(undo) = self.undos.pop() else {
            return Err(MoveError::NoUndos);
        };

        let res = self.state.apply_undo(undo);
        if res.is_ok() {
            self.moves += 1;
        }

        res
    }

    pub(crate) fn moves(&self) -> u32 {
        self.moves
    }

    pub(crate) fn undos(&self) -> usize {
        self.undos.len()
    }

    pub(crate) fn state(&self) -> &GameState {
        &self.state
    }
}
