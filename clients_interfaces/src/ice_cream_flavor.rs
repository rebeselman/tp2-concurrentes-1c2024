//! Supported flavors for ice cream

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy,Serialize, Deserialize)]
pub enum IceCreamFlavor {
    Chocolate,
    Strawberry,
    Vanella,
    Mint,
    Lemon
}


impl IceCreamFlavor {
    /// Returns all the possible values of IceCreamFlavor
    pub fn values()-> Vec<IceCreamFlavor> {
        return vec![IceCreamFlavor::Chocolate, IceCreamFlavor::Strawberry, IceCreamFlavor::Vanella, IceCreamFlavor::Mint, IceCreamFlavor::Lemon]
    }
}