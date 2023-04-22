//! Contenedor de recursos de la cafetera

/// Representa a un contenedor de ingredientes.
/// Tiene como estado la cantidad que le queda de recurso, cuanto se consumio,
/// y si se acabo la reposicion del contenedor.
pub struct Container {
    pub remaining: u64,
    pub consumed: u64,
    pub finished: bool,
}

impl Container {
    pub fn new(initial_capacity: u64) -> Container {
        Container {
            remaining: initial_capacity,
            consumed: 0,
            finished: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }
}
