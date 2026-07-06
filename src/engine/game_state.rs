use crate::engine::{
    card::{Card, Rank, Suit, get_deck},
    errors::MoveError,
};
use rand::{RngCore, SeedableRng, rngs::StdRng, seq::SliceRandom};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Move {
    Draw,
    Recycle,
    WasteToFoundation {
        foundation_idx: usize,
    },
    WasteToTableau {
        tableau_idx: usize,
    },
    TableauToFoundation {
        tableau_idx: usize,
        foundation_idx: usize,
    },
    FoundationToTableau {
        foundation_idx: usize,
        tableau_idx: usize,
    },
    TableauToTableau {
        src_tableau_idx: usize,
        card_idx: usize,
        dst_tableau_idx: usize,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Undo {
    applied: Move,
    moved_count: usize,
    flipped_tableau_top: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub(crate) struct GameState {
    stock: Vec<Card>,
    waste: Vec<Card>,
    foundations: [Vec<Card>; 4],
    tableaus: [Vec<Card>; 7],
    seed: u64,
}

impl GameState {
    pub(crate) fn from_seed(seed: u64) -> Self {
        let mut deck = get_deck();

        let mut rng = StdRng::seed_from_u64(seed);
        deck.shuffle(&mut rng);

        let mut state = Self {
            seed: seed,
            ..Default::default()
        };

        for col in 0..7 {
            for row in 0..=col {
                let mut card = deck.pop().unwrap();
                card.revealed = row == col;
                state.tableaus[col].push(card);
            }
        }

        for card in deck.drain(..) {
            state.stock.push(card);
        }

        state
    }

    pub(crate) fn new_random() -> Self {
        Self::from_seed(StdRng::from_os_rng().next_u64())
    }

    pub(crate) fn hint_move(&self) -> Option<Move> {
        let mut hint = None;
        self.visit_legal_moves(|mv| {
            hint = Some(mv);
            false
        });

        hint
    }

    pub(crate) fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        self.visit_legal_moves(|mv| {
            moves.push(mv);
            true
        });

        moves
    }

    pub(super) fn apply_move(&mut self, mv: Move) -> Result<Undo, MoveError> {
        let mut flipped_tableau_top = false;
        let mut moved_count = 0;

        match mv {
            Move::Draw => {
                let Some(mut card) = self.stock.pop() else {
                    return Err(MoveError::StockEmpty);
                };

                card.revealed = true;

                self.waste.push(card);
            }
            Move::Recycle => {
                if !self.stock.is_empty() {
                    return Err(MoveError::StockNotEmpty);
                }

                while let Some(mut card) = self.waste.pop() {
                    card.revealed = false;
                    self.stock.push(card);
                }
            }
            Move::WasteToFoundation { foundation_idx } => {
                let Some(card) = self.waste.last() else {
                    return Err(MoveError::WasteEmpty);
                };

                let foundation = &mut self.foundations[foundation_idx];
                let foundation_top = foundation.last();
                if !can_place_on_foundation(card, foundation_top) {
                    return Err(MoveError::CantPlaceOnFoundation);
                }

                let card = self.waste.pop().unwrap();
                foundation.push(card);
            }
            Move::WasteToTableau { tableau_idx } => {
                let Some(card) = self.waste.last() else {
                    return Err(MoveError::WasteEmpty);
                };

                let tableau = &mut self.tableaus[tableau_idx];
                let tableau_top = tableau.last();
                if !can_place_on_tableau(card, tableau_top) {
                    return Err(MoveError::CantPlaceOnTableau);
                }

                let card = self.waste.pop().unwrap();
                tableau.push(card);
            }
            Move::TableauToFoundation {
                tableau_idx,
                foundation_idx,
            } => {
                let tableau = &mut self.tableaus[tableau_idx];
                let Some(card) = tableau.last() else {
                    return Err(MoveError::TableauEmpty);
                };

                let foundation = &mut self.foundations[foundation_idx];
                let foundation_top = foundation.last();
                if !can_place_on_foundation(card, foundation_top) {
                    return Err(MoveError::CantPlaceOnFoundation);
                }

                let card = tableau.pop().unwrap();
                foundation.push(card);

                if let Some(card) = tableau.last_mut()
                    && !card.revealed
                {
                    flipped_tableau_top = true;
                    card.revealed = true;
                }
            }
            Move::FoundationToTableau {
                foundation_idx,
                tableau_idx,
            } => {
                let foundation = &mut self.foundations[foundation_idx];
                let Some(card) = foundation.last() else {
                    return Err(MoveError::FoundationEmpty);
                };

                if card.rank == Rank::Ace {
                    return Err(MoveError::CantPlaceAceBackToTableau);
                }

                let tableau = &mut self.tableaus[tableau_idx];
                let tableau_top = tableau.last();
                if !can_place_on_tableau(card, tableau_top) {
                    return Err(MoveError::CantPlaceOnTableau);
                }

                let card = foundation.pop().unwrap();
                tableau.push(card);
            }
            Move::TableauToTableau {
                src_tableau_idx,
                card_idx,
                dst_tableau_idx,
            } => {
                if src_tableau_idx == dst_tableau_idx {
                    return Err(MoveError::SameSourceAndDestination);
                }

                let (src_tableau, dst_tableau) = if src_tableau_idx < dst_tableau_idx {
                    let (left, right) = self.tableaus.split_at_mut(dst_tableau_idx);
                    (&mut left[src_tableau_idx], &mut right[0])
                } else {
                    let (left, right) = self.tableaus.split_at_mut(src_tableau_idx);
                    (&mut right[0], &mut left[dst_tableau_idx])
                };

                let Some(cards) = src_tableau.get(card_idx..) else {
                    return Err(MoveError::InvalidCardIndex);
                };

                if !can_grab_from_tableau(cards) {
                    return Err(MoveError::InvalidCardsToGrab);
                }

                let dst_tableau_top = dst_tableau.last();
                if !can_place_on_tableau(&cards[0], dst_tableau_top) {
                    return Err(MoveError::CantPlaceOnTableau);
                }

                moved_count = src_tableau.len() - card_idx;

                let cards = src_tableau.drain(card_idx..);
                dst_tableau.extend(cards);

                if let Some(card) = src_tableau.last_mut()
                    && !card.revealed
                {
                    flipped_tableau_top = true;
                    card.revealed = true;
                }
            }
        };

        Ok(Undo {
            applied: mv,
            moved_count: moved_count,
            flipped_tableau_top: flipped_tableau_top,
        })
    }

    pub(super) fn apply_undo(&mut self, undo: Undo) -> Result<(), MoveError> {
        match undo.applied {
            Move::Draw => {
                let Some(mut card) = self.waste.pop() else {
                    return Err(MoveError::WasteEmpty);
                };

                card.revealed = false;

                self.stock.push(card);
            }
            Move::Recycle => {
                if !self.waste.is_empty() {
                    return Err(MoveError::WasteNotEmpty);
                }

                while let Some(mut card) = self.stock.pop() {
                    card.revealed = true;
                    self.waste.push(card);
                }
            }
            Move::WasteToFoundation { foundation_idx } => {
                let Some(card) = self.foundations[foundation_idx].pop() else {
                    return Err(MoveError::FoundationEmpty);
                };

                self.waste.push(card);
            }
            Move::WasteToTableau { tableau_idx } => {
                let Some(card) = self.tableaus[tableau_idx].pop() else {
                    return Err(MoveError::TableauEmpty);
                };

                self.waste.push(card);
            }
            Move::TableauToFoundation {
                tableau_idx,
                foundation_idx,
            } => {
                let Some(card) = self.foundations[foundation_idx].pop() else {
                    return Err(MoveError::FoundationEmpty);
                };

                let tableau = &mut self.tableaus[tableau_idx];
                tableau.push(card);

                if undo.flipped_tableau_top {
                    let tableau_len = tableau.len();
                    let Some(card) = tableau.get_mut(tableau_len - 2) else {
                        return Err(MoveError::InvalidFlippedTableauTopValue);
                    };

                    card.revealed = false;
                }
            }
            Move::FoundationToTableau {
                foundation_idx,
                tableau_idx,
            } => {
                let Some(card) = self.tableaus[tableau_idx].pop() else {
                    return Err(MoveError::TableauEmpty);
                };

                self.foundations[foundation_idx].push(card);
            }
            Move::TableauToTableau {
                src_tableau_idx,
                card_idx,
                dst_tableau_idx,
            } => {
                if src_tableau_idx == dst_tableau_idx {
                    return Err(MoveError::SameSourceAndDestination);
                }

                let (src_tableau, dst_tableau) = if src_tableau_idx < dst_tableau_idx {
                    let (left, right) = self.tableaus.split_at_mut(dst_tableau_idx);
                    (&mut left[src_tableau_idx], &mut right[0])
                } else {
                    let (left, right) = self.tableaus.split_at_mut(src_tableau_idx);
                    (&mut right[0], &mut left[dst_tableau_idx])
                };

                // TODO use a check instead of saturating sub
                let start = dst_tableau.len().saturating_sub(undo.moved_count);

                let cards = dst_tableau.drain(start..);
                src_tableau.extend(cards);

                if undo.flipped_tableau_top {
                    // TODO use a check instead of saturating sub
                    let Some(card) = src_tableau.get_mut(card_idx.saturating_sub(1)) else {
                        return Err(MoveError::InvalidFlippedTableauTopValue);
                    };

                    card.revealed = false;
                }
            }
        }

        Ok(())
    }

    fn is_safe_to_foundation(&self, card: &Card) -> bool {
        let rank = card.rank_u8();
        if rank <= 1 {
            return true;
        }

        let mut tops = [None; 4];
        for foundation in self.foundations_iter() {
            if let Some(card) = foundation.last() {
                tops[card.suit as usize] = Some(card.rank_u8());
            }
        }

        let same_color_other_suit = match card.suit {
            Suit::Diamonds => Suit::Heart as usize,
            Suit::Heart => Suit::Diamonds as usize,
            Suit::Spades => Suit::Club as usize,
            Suit::Club => Suit::Spades as usize,
        };

        let (opposite_suit1, opposite_suit2) = if card.is_red() {
            (Suit::Spades as usize, Suit::Club as usize)
        } else {
            (Suit::Diamonds as usize, Suit::Heart as usize)
        };

        let opposite_ok = tops[opposite_suit1].is_some_and(|r| r + 2 >= rank)
            && tops[opposite_suit2].is_some_and(|r| r + 2 >= rank);
        let same_color_ok = tops[same_color_other_suit].is_some_and(|r| r + 3 >= rank);

        opposite_ok && same_color_ok
    }

    /// `only_safe` determines which moves to look at
    /// [`true`] means return only safe moves
    /// [`false`] means return only unsafe moves
    fn visit_legal_foundation_moves(
        &self,
        only_safe: bool,
        proceed: &mut impl FnMut(Move) -> bool,
    ) {
        if let Some(card) = self.waste.last() {
            for (idx, foundation) in self.foundations_iter().enumerate() {
                if !can_place_on_foundation(card, foundation.last()) {
                    continue;
                }

                let safe = self.is_safe_to_foundation(card);
                if safe == only_safe
                    && !proceed(Move::WasteToFoundation {
                        foundation_idx: idx,
                    })
                {
                    return;
                }
            }
        }

        // tableau tops
        for (tableau_idx, tableau) in self.tableaus_iter().enumerate() {
            let Some(card) = tableau.last() else {
                continue;
            };

            if !card.revealed {
                continue;
            }

            for (foundation_idx, foundation) in self.foundations_iter().enumerate() {
                if !can_place_on_foundation(card, foundation.last()) {
                    continue;
                }

                let safe = self.is_safe_to_foundation(card);
                if safe == only_safe
                    && !proceed(Move::TableauToFoundation {
                        tableau_idx: tableau_idx,
                        foundation_idx: foundation_idx,
                    })
                {
                    return;
                }
            }
        }
    }

    fn visit_legal_moves(&self, mut proceed: impl FnMut(Move) -> bool) {
        self.visit_legal_foundation_moves(true, &mut proceed);

        // tableau to tableau
        for (src_tableau_idx, src_tableau) in self.tableaus_iter().enumerate() {
            if src_tableau.is_empty() {
                continue;
            }

            // find first revealed card
            // if tableau is not empty, there should always be a revealed card
            let revealed_idx = src_tableau.iter().position(|c| c.revealed).unwrap();

            for card_idx in revealed_idx..src_tableau.len() {
                let cards = &src_tableau[card_idx..];
                if !can_grab_from_tableau(cards) {
                    continue;
                }

                for (dst_tableau_idx, dst_tableau) in self.tableaus_iter().enumerate() {
                    if dst_tableau_idx == src_tableau_idx {
                        continue;
                    }

                    if can_place_on_tableau(&cards[0], dst_tableau.last())
                        && !proceed(Move::TableauToTableau {
                            src_tableau_idx: src_tableau_idx,
                            card_idx: card_idx,
                            dst_tableau_idx: dst_tableau_idx,
                        })
                    {
                        return;
                    }
                }
            }
        }

        // unsafe foundation moves (waste to foundation, tableau to foundation)
        self.visit_legal_foundation_moves(false, &mut proceed);

        // waste to tableau
        if let Some(card) = self.waste_top() {
            for (idx, tableau) in self.tableaus_iter().enumerate() {
                if can_place_on_tableau(card, tableau.last())
                    && !proceed(Move::WasteToTableau { tableau_idx: idx })
                {
                    return;
                }
            }
        }

        // foundation to Tableau
        for (foundation_idx, foundation) in self.foundations_iter().enumerate() {
            let Some(card) = foundation.last() else {
                continue;
            };

            if card.rank == Rank::Ace {
                continue;
            }

            for (tableau_idx, tableau) in self.tableaus_iter().enumerate() {
                if can_place_on_tableau(card, tableau.last())
                    && !proceed(Move::FoundationToTableau {
                        foundation_idx: foundation_idx,
                        tableau_idx: tableau_idx,
                    })
                {
                    return;
                }
            }
        }

        // draw or recycle
        if !self.stock.is_empty() {
            _ = proceed(Move::Draw);
        } else if !self.waste.is_empty() {
            _ = proceed(Move::Recycle);
        }
    }

    pub(crate) fn is_won(&self) -> bool {
        self.foundations.iter().all(|f| f.len() == 13)
    }

    pub(crate) fn stock_len(&self) -> usize {
        self.stock.len()
    }

    pub(crate) fn waste_top(&self) -> Option<&Card> {
        self.waste.last()
    }

    pub(crate) fn waste(&self) -> &[Card] {
        &self.waste
    }

    pub(crate) fn waste_len(&self) -> usize {
        self.waste.len()
    }

    pub(crate) fn foundation_top(&self, idx: usize) -> Option<&Card> {
        self.foundations.get(idx).and_then(|f| f.last())
    }

    pub(crate) fn foundation(&self, idx: usize) -> &[Card] {
        self.foundations
            .get(idx)
            .map(|f| f.as_slice())
            .unwrap_or(&[])
    }

    pub(crate) fn foundation_len(&self, idx: usize) -> usize {
        self.foundations.get(idx).map(|f| f.len()).unwrap_or(0)
    }

    pub(crate) fn foundations_iter(&self) -> impl Iterator<Item = &[Card]> {
        self.foundations.as_slice().iter().map(|t| t.as_slice())
    }

    pub(crate) fn tableau(&self, idx: usize) -> &[Card] {
        self.tableaus.get(idx).map(|t| t.as_slice()).unwrap_or(&[])
    }

    pub(crate) fn tableaus_iter(&self) -> impl Iterator<Item = &[Card]> {
        self.tableaus.as_slice().iter().map(|t| t.as_slice())
    }

    pub(crate) fn seed(&self) -> u64 {
        self.seed
    }

    /// Dev-only: flip every tableau card face-up (auto-complete testing).
    #[cfg(debug_assertions)]
    pub(crate) fn reveal_all_tableau(&mut self) {
        for pile in self.tableaus.iter_mut() {
            for card in pile.iter_mut() {
                card.revealed = true;
            }
        }
    }
}

pub(crate) fn can_grab_from_tableau(cards: &[Card]) -> bool {
    if cards.is_empty() {
        return false;
    }

    for pair in cards.windows(2) {
        let first: Card = pair[0];
        let second: Card = pair[1];

        if !first.revealed || !second.revealed {
            return false;
        }

        if first.rank_u8() != second.rank_u8() + 1 {
            return false;
        }

        if !first.color_differs(&second) {
            return false;
        }
    }

    true
}

pub(crate) fn can_place_on_foundation(card: &Card, foundation_top: Option<&Card>) -> bool {
    if !card.revealed {
        return false;
    }

    match foundation_top {
        None => card.rank == Rank::Ace,
        Some(top) => card.suit == top.suit && card.rank_u8() == top.rank_u8() + 1,
    }
}

pub(crate) fn can_place_on_tableau(card: &Card, tableau_top: Option<&Card>) -> bool {
    if !card.revealed {
        return false;
    }

    match tableau_top {
        None => card.rank == Rank::King,
        Some(top) => card.color_differs(top) && top.rank_u8() == card.rank_u8() + 1,
    }
}
