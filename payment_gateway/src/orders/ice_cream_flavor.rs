//! Supported flavors for ice cream

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum IceCreamFlavor {
    Chocolate,
    Strawberry,
    Vanilla,
    Mint,
    Lemon,
}
