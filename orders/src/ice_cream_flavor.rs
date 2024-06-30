//! Supported flavors for ice cream

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IceCreamFlavor {
    Chocolate,
    Strawberry,
    Vanilla,
    Mint,
    Lemon,
}

impl IceCreamFlavor {
    /// Returns all the possible values of IceCreamFlavor
    pub fn values() -> Vec<IceCreamFlavor> {
        vec![
            IceCreamFlavor::Chocolate,
            IceCreamFlavor::Strawberry,
            IceCreamFlavor::Vanilla,
            IceCreamFlavor::Mint,
            IceCreamFlavor::Lemon,
        ]
    }
}
