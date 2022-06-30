use ic_cdk::api::stable;
use crate::types::*;

pub fn stable_grow_memory_page(page_count: u64) -> Result<(), KvError> {
    if page_count > 100 || page_count <= 0 {
        return Err(KvError::Other(format!("param err, page_count:{}", page_count)));
    }

    match stable::stable64_grow(page_count) {
        Ok(..) => { Ok(()) }
        Err(err) => Err(KvError::Other(format!("{}", err)))
    }
}

fn get_stable64_size() -> u64 {
    stable::stable64_size()
}
