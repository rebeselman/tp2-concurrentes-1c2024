//! Represents an item of an order of ice cream
use serde::{Deserialize, Serialize};

use crate::{container_type::ContainerType, ice_cream_flavor::IceCreamFlavor};

/// Contains the units of the item, the type of container and the ice cream flavor's requested
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Item {
    container: ContainerType,
    units: u32,
    flavors: Vec<IceCreamFlavor>,
}

impl Item {
    /// Creates a new item
    /// # Arguments
    /// * `container` - A ContainerType representing the type of container
    /// * `units` - A u32 representing the units of the item
    /// * `flavors` - A Vec<IceCreamFlavor> representing the flavors of the ice cream
    /// # Returns
    /// * An Item
    pub fn new(container: ContainerType, units: u32, flavors: Vec<IceCreamFlavor>) -> Item {
        Item {
            container,
            units,
            flavors,
        }
    }

    /// To obtain the container of this item
    pub fn container(&self) -> &ContainerType {
        &self.container
    }

    /// To obtain the units of this item
    pub fn units(&self) -> u32 {
        self.units
    }

    /// To obtain the flavors of this item
    pub fn flavors(&self) -> &Vec<IceCreamFlavor> {
        &self.flavors
    }

    /// Time to prepare one item should be based on container type
    pub fn time_to_prepare(&self) -> u32 {
        match self.container {
            ContainerType::Cup => 200,
            ContainerType::Cone => 100,
            ContainerType::OneKilo => 1000,
            ContainerType::HalfKilo => 500,
            ContainerType::QuarterKilo => 300,

        }
    }
}
