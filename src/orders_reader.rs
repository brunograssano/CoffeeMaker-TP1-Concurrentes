pub(crate) mod orders_reader {
    use std::error::Error;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use serde::Deserialize;

    use crate::order::order::Order;

    #[derive(Deserialize)]
    struct OrdersConfiguration {
        orders: Vec<Order>,
    }

    pub fn read_orders_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Order>, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let orders_config: OrdersConfiguration = serde_json::from_reader(reader)?;
        Ok(orders_config.orders)
    }
}