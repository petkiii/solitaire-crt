#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) enum Suit {
    Diamonds = 0,
    Spades,
    Heart,
    Club,
}

impl Suit {
    const fn values() -> &'static [Suit] {
        &[Suit::Diamonds, Suit::Spades, Suit::Heart, Suit::Club]
    }

    const fn label(self) -> &'static str {
        match self {
            Suit::Spades => "♠",
            Suit::Club => "♣",
            Suit::Diamonds => "♦",
            Suit::Heart => "♥",
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Rank {
    Ace = 0,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    const fn values() -> &'static [Rank] {
        &[
            Rank::Ace,
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
        ]
    }

    const fn label(self) -> &'static str {
        match self {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Card {
    pub(crate) suit: Suit,
    pub(crate) rank: Rank,
    pub(crate) revealed: bool,
}

impl Card {
    pub fn is_red(self) -> bool {
        matches!(self.suit, Suit::Diamonds | Suit::Heart)
    }

    pub(crate) fn color_differs(self, other: &Card) -> bool {
        self.is_red() != other.is_red()
    }

    pub(crate) fn rank(self) -> Rank {
        self.rank
    }

    pub(crate) fn rank_u8(self) -> u8 {
        self.rank as u8
    }

    pub(crate) fn suit(self) -> Suit {
        self.suit
    }

    fn suit_u8(self) -> u8 {
        self.suit as u8
    }

    pub(crate) fn rank_str(self) -> &'static str {
        self.rank.label()
    }

    pub(crate) fn suit_str(self) -> &'static str {
        self.suit.label()
    }

    pub(crate) fn revealed(self) -> bool {
        self.revealed
    }

    pub(crate) fn id(self) -> u8 {
        self.suit_u8() * 13 + self.rank_u8()
    }
}

pub(crate) fn get_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);

    for &suit in Suit::values() {
        for &rank in Rank::values() {
            deck.push(Card {
                suit,
                rank,
                revealed: false,
            });
        }
    }

    deck
}
