pub mod order {
    pub const TOTAL_INGREDIENTS: usize = 6;
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum Ingredient {
        GroundCoffee,
        HotWater,
        Cacao,
        MilkFoam,
        GrainsToGrind,
        ColdMilk,
    }

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
}