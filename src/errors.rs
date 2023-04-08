#[derive(Debug)]
pub enum DispenserError {
    LockError,
    EmptyQueueWhenNotExpected,
    IngredientNotInMap,
}

impl<T> From<std::sync::PoisonError<T>> for DispenserError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        DispenserError::LockError
    }
}