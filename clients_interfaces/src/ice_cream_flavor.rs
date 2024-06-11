//! Supported flavors for ice cream

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub enum IceCreamFlavor {
    Chocolate,
    Strawberry,
    Vanella,
    Mint,
    Lemon
}