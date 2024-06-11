//! Types of Ice Cream Container

use serde::{Deserialize, Serialize};
// no se cómo traducir esto a inglés :D
#[derive(Serialize, Deserialize)]
pub enum ContainerType {
    Cup,
    Cone,
    OneKilo,
    HalfKilo,
    QuarterKilo,
}