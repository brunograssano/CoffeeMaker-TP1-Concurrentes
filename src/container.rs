pub struct Container {
    pub remaining: u64,
    pub consumed: u64,
    pub finished: bool,
}

impl Container {
    pub fn new(initial_capacity: u64) -> Container {
        Container { remaining: initial_capacity, consumed: 0, finished: false }
    }
}