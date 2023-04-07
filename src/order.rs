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
        pub ingredients: Vec<Ingredient>,
    }
}