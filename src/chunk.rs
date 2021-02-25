use serde::{Serialize, Deserialize};
use std::cmp::PartialEq;
use std::fmt::Debug;
use crate::types::DisplayMap;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Chunk {
    data: DisplayMap,
}
