//! Represents an item of an order of ice cream

use super::{container_type::ContainerType, ice_cream_flavor::IceCreamFlavor};
use serde::{Deserialize, Serialize};

/// Contains the units of the item, the type of container and the ice cream flavor's requested
#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    container: ContainerType,
    units: u32,
    flavors: Vec<IceCreamFlavor>,
}
