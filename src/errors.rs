#[derive(Debug)]
pub enum DispenserError {
    LockError,
    EmptyQueueWhenNotExpected,
    IngredientNotInMap,
}

pub enum ReplenisherError {
    LockError,
}

pub enum CoffeeMakerError {
    JoinError,
    IngredientNotInMap,
}

impl<T> From<std::sync::PoisonError<T>> for DispenserError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        DispenserError::LockError
    }
}

impl<T> From<std::sync::PoisonError<T>> for ReplenisherError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        ReplenisherError::LockError
    }
}