#[derive(Clone)]
pub struct Container {
    quantity: usize,
    in_use_by: Option<usize>,
}

impl Container {
    pub fn new(quantity: usize) -> Self {
        Container {
            quantity,
            in_use_by: None,
        }
    }

    pub fn is_available(&self) -> bool {
        self.in_use_by.is_none()
    }

    pub fn quantity(&self) -> usize {
        self.quantity
    }

    pub fn use_container(&mut self, robot_id: usize, amount: usize) {
        self.in_use_by = Some(robot_id);
        self.quantity -= amount;
    }

    pub fn release_container(&mut self) {
        self.in_use_by = None;
    }
}