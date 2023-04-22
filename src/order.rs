//! Representacion de un pedido

/// Cantidad total de ingredientes unicos que maneja la cafetera. Debe de coincidir con la cantidad en el `enum Ingredient`
pub const TOTAL_INGREDIENTS: usize = 6;

/// Recursos que puede manejar la cafetera
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Ingredient {
    GroundCoffee,
    HotWater,
    Cacao,
    MilkFoam,
    GrainsToGrind,
    ColdMilk,
}

/// Estructura para representar un pedido.
/// Esta compuesta por un id y un vector con los ingredientes y cantidades a usar. El vector no sigue un orden en particular
#[derive(Debug)]
pub struct Order {
    pub id: usize,
    pub ingredients: Vec<(Ingredient, u64)>,
}

impl Order {
    pub fn new(id: usize, ingredients: Vec<(Ingredient, u64)>) -> Order {
        Order { id, ingredients }
    }
}
