/// Codigos de errores de la cafetera
#[derive(Debug, PartialEq, Eq)]
pub enum CoffeeMakerError {
    /// Indica que no se encontro un ingrediente requerido en el mapa de recursos
    IngredientNotInMap,

    /// Indica que ocurrio un error con un lock. Se puede dar en caso de que este envenenado
    LockError,

    /// Indica que al buscar un pedido en la cola se encontro con que estaba vacia cuando no deberia
    EmptyQueueWhenNotExpected,

    /// Ocurrio un error en la lectura del archivo, ya sea porque no existe o tiene un formato equivocado
    FileReaderError,
}

impl<T> From<std::sync::PoisonError<T>> for CoffeeMakerError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        CoffeeMakerError::LockError
    }
}
