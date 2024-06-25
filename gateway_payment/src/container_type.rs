//! Types of Ice Cream Container

use std::str::FromStr;
use serde::{Deserialize, Serialize};
// no se cómo traducir esto a inglés :D
#[derive(Serialize, Deserialize, Debug)]
pub enum ContainerType {
    Cup,
    Cone,
    OneKilo,
    HalfKilo,
    QuarterKilo,
}

// impl FromStr for ContainerType {
//     type Err = ();
//
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         // String to lowercase
//         let s = s.to_lowercase();
//         match s.as_str() {
//             "cone" => Ok(ContainerType::Cone),
//             "cup" => Ok(ContainerType::Cup),
//             "1 kg" => Ok(ContainerType::OneKilo),
//             "1/2 kg" => Ok(ContainerType::HalfKilo),
//             "1/4 kg" => Ok(ContainerType::QuarterKilo),
//             _ => Err(()),
//         }
//     }
// }