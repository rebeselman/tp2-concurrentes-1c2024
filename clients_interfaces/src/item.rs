//! Represents an item of an order of ice cream
use serde::{Deserialize, Serialize};

use crate::{container_type::ContainerType, ice_cream_flavor::IceCreamFlavor};

/// Contains the units of the item, the type of container and the ice cream flavor's requested
#[derive(Serialize, Deserialize)]
pub struct Item {
    container: ContainerType,
    units: u32,
    flavors: Vec<IceCreamFlavor>
}