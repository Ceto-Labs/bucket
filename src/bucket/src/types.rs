use std::collections::HashMap;
use ic_cdk::export::candid::{CandidType};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Deserialize, Debug, Clone)]
pub enum KvError {
    InsufficientMemory,
    BlobSizeError,
    InvalidKey,
    Other(String),
}

#[derive(CandidType, Serialize, Default, Debug, Clone)]
pub struct Kv {
    pub(crate) kv_set : HashMap<String,(Vec<u64>,u64)>, //key :(blocks/position)
}


// #[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
#[derive(CandidType, Default, Deserialize)]
pub struct Layout {
    pub(crate) stable_blocks_count: u64,
    pub(crate) bit_map: Vec<u8>,//8*10>>30/512
}
