pub struct Container {
    pub remaining: u64,
    pub consumed: u64,
}

impl Container {
    pub fn new(initial_capacity: u64) -> Container {
        Container { remaining: initial_capacity, consumed: 0 }
    }
}