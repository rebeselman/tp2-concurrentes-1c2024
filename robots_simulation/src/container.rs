#[derive(Clone)]
pub struct Container {
    quantity: u32,
    in_use_by: Option<usize>,
}

impl Container {
    pub fn new(quantity: u32) -> Self {
        Container {
            quantity,
            in_use_by: None,
        }
    }

    pub fn is_available(&self) -> bool {
        self.in_use_by.is_none()
    }

    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    pub fn use_container(&mut self, robot_id: usize, amount: &u32) {
        self.in_use_by = Some(robot_id);
        self.quantity -= amount;
        println!("Container in use by robot {}. Available quantity: {}", robot_id, self.quantity);
    }

    pub fn release_container(&mut self) {
        self.in_use_by = None;
    }
}