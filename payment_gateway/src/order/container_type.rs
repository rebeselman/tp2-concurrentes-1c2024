//! Types of Ice Cream Container

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ContainerType {
    Cup,
    Cone,
    OneKilo,
    HalfKilo,
    QuarterKilo,
}
