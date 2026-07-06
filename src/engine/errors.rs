use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum MoveError {
    // Move errors
    #[error("stock is empty")]
    StockEmpty,
    #[error("stock is not empty")]
    StockNotEmpty,
    #[error("waste is empty")]
    WasteEmpty,
    #[error("waste is not empty")]
    WasteNotEmpty,
    #[error("tableau is empty")]
    TableauEmpty,
    #[error("foundation is empty")]
    FoundationEmpty,
    #[error("can't place on foundation")]
    CantPlaceOnFoundation,
    #[error("can't place on tableau")]
    CantPlaceOnTableau,
    #[error("can't place ace back to tableau")]
    CantPlaceAceBackToTableau,
    #[error("src is the same as dst")]
    SameSourceAndDestination,
    #[error("invalid card index")]
    InvalidCardIndex,
    #[error("invalid cards to grab")]
    InvalidCardsToGrab,
    #[error("invalid flipped tableau top value")]
    InvalidFlippedTableauTopValue,
    #[error("no undos")]
    NoUndos,

    // Auto move errors
    #[error("no valid move for waste card")]
    NoValidMoveForWasteCard,
    #[error("no valid auto-move destination tableau")]
    NoValidAutoMoveDestinationTableau,
    #[error("no valid tableau destination")]
    NoValidTableauDestination,
}
