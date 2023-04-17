#[derive(Debug, PartialEq, Eq)]
pub enum CoffeeMakerError {
    JoinError,
    IngredientNotInMap,
    LockError,
    EmptyQueueWhenNotExpected,
    FileReaderError,
}

impl<T> From<std::sync::PoisonError<T>> for CoffeeMakerError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        CoffeeMakerError::LockError
    }
}
