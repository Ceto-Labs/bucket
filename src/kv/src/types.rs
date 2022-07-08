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

#[derive(CandidType, Serialize, Deserialize, Default, Debug, Clone)]
pub struct Kv {
    pub(crate) kv_set: HashMap<String, (Vec<u64>, u64)>, //key :(blocks/position)
}

#[derive(CandidType, Deserialize, Serialize, Clone)]
pub struct Layout {
    pub(crate) stable_blocks_count: u64,
    pub(crate) bit_map: Vec<u8>,
    pub(crate) kv_block_size : u64,
}

#[derive(CandidType, Default, Deserialize, Serialize)]
pub struct StableStruct {
    pub(crate) layout: Layout,
    pub(crate) kv: Kv,
}




