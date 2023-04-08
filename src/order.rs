pub mod order {
    #[derive(Debug)]
    pub enum Ingredient {
        GroundCoffee(u64),
        HotWater(u64),
        Cacao(u64),
        MilkFoam(u64),
    }

    #[derive(Debug)]
    pub struct Order {
        pub id: usize,
        pub ingredients: Vec<Ingredient>,
    }

    impl Order {
        pub fn new(id: usize, ingredients: Vec<Ingredient>) -> Order {
            Order { id, ingredients }
        }
    }
}