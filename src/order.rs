pub mod order {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct Order {
        ground_coffee: u64,
        hot_water: u64,
        cacao: u64,
        milk_foam: u64,
    }
}