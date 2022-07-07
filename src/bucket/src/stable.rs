use ic_cdk::api::stable;
use crate::types::*;
use ic_cdk::api;

pub static RESERVED_PAGE: u64 = 320;

pub fn stable_grow_memory_page(page_count: u64) -> Result<(), KvError> {
    // if page_count <= 0 {
    //     return Err(KvError::Other(format!("param err, page_count:{}", page_count)));
    // }

    let mut page_count = page_count;
    if stable::stable64_size() == 0 { page_count += RESERVED_PAGE }

    api::print(format!("grow memory:{}", page_count));
    match stable::stable64_grow(page_count) {
        Ok(..) => { Ok(()) }
        Err(err) => Err(KvError::Other(format!("{}", err)))
    }
}

pub fn storage_data(start: u64, data: Vec<u8>) {
    stable::stable64_write(start, data.as_slice());
}

pub fn load_from_stable(position: u64, len: usize) -> Vec<u8> {
    let mut buf = [0].repeat(len as usize);
    stable::stable64_read(position, &mut buf);
    buf.clone()
}