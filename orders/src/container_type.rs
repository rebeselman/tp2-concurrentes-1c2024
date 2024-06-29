//! Types of Ice Cream Container

use serde::{Deserialize, Serialize};
// no se cómo traducir esto a inglés :D
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContainerType {
    Cup,
    Cone,
    OneKilo,
    HalfKilo,
    QuarterKilo,
}



impl ContainerType {
    /// Returns all the possible values of ContainerType
    pub fn values()-> Vec<ContainerType> {
        return vec![ContainerType::Cup, ContainerType::Cone, ContainerType::OneKilo, ContainerType::HalfKilo, ContainerType::QuarterKilo]
    }
}