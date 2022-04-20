use ic_cdk::api;
use ic_cdk::api::{stable, trap};
use ic_cdk::export::candid::{candid_method, CandidType, Nat, Decode, Encode};
use num_traits::ToPrimitive;
use std::cell::RefCell;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};
use serde::{Deserialize, Serialize};

mod bucket;

use bucket::Bucket;

#[derive(CandidType, Deserialize, Serialize, Clone)]
struct NftInfo {
    read_offset: u64,
    write_offset: u64,
    nft_data: Vec<u8>,
}


impl Default for NftInfo {
    fn default() -> Self {
        NftInfo {
            read_offset: 0,
            nft_data: vec![],
            write_offset: 0,
        }
    }
}

thread_local!(
    /* stable */   static NFTINFO: RefCell<NftInfo> = RefCell::new(NftInfo::default());
);

#[init]
#[candid_method(init)]
fn init() {
    NFTINFO.with(|nftinfo| {
        let data = [0].repeat(10485760);
        nftinfo.borrow_mut().nft_data.extend(data.iter())
    });

}

#[ic_cdk_macros::query]
fn greet(name: String) -> String {
    format!("Hello 335566, {}!", name)
}

// NOTE:
// If you plan to store gigabytes of state and upgrade the code,
// Using stable memory as the main storage is a good option to consider
#[pre_upgrade]
fn pre_upgrade() {
    let _nftinfo = NFTINFO.with(|nftinfo| {
        nftinfo.borrow().clone()
    });

    let bytes = bincode::serialize::<NftInfo>(&_nftinfo).unwrap();

    // let bytes = Encode!(&_nftinfo).unwrap();
    Bucket::pre_upgrade(bytes);
}

#[post_upgrade]
fn post_upgrade() {
    let bytes = Bucket::post_upgrade();

    NFTINFO.with(|nftinfo| {
        *nftinfo.borrow_mut() = bincode::deserialize(&bytes).unwrap()
        // *nftinfo.borrow_mut() = Decode!(&bytes, NftInfo).unwrap();
    });
}

////////////////////////////////////////////////////////////
// test upgrade
#[update(name = "uploadData")]
#[candid_method(update, rename = "uploadData")]
fn upload_data(num: Nat) {
    let num = num.0.to_u32().unwrap();
    let mut i = 1;
    while i <= num {
        let mut j = 0;
        while j < 16 {
            let data: Vec<u8> = vec![i as u8; (64 * 64) as usize];
            let name = format!("{}.png", i as u8);
            let ret = Bucket::put(name, data);
            match ret {
                Ok(..) => {}
                Err(err) => {
                    api::print(format!("upload tt7744tttttn9 66 data err:{:?}", err));
                    assert!(false)
                }
            }
            j += 1;
        }
        i += 1;
    }
}

#[update(name = "uploadBigData")]
#[candid_method(update, rename = "uploadBigData")]
fn upload_big_data(num: u32) {
    let data: Vec<u8> = vec![1 as u8; (num * 1024 * 1024) as usize];

    let name = format!("{}.png", 0);

    let ret = Bucket::put(name, data);
    match ret {
        Ok(..) => {}
        Err(err) => api::print(format!("upload data  err:{:?}", err)),
    }
}

#[update(name = "checkData")]
#[candid_method(update, rename = "checkData")]
fn check_data(start: u8, end: u8) {
    assert!(start <= end);

    let mut i = start;
    while i <= end {
        let name = format!("{}.png", i as u8);
        api::print(name.clone());
        let ret = Bucket::get(name);
        match ret {
            Ok(data) => {
                for v in &data {
                    api::print(format!(
                        "data : start: {}, end :{}, len:{}",
                        v[0],
                        v[v.len() - 1],
                        v.len()
                    ));
                    if v[0] != i as u8 || v[v.len() - 1] != i as u8 {
                        api::print(format!("vec:{:?}", v));
                        assert!(false);
                    }
                }
            }
            Err(err) => {
                api::print(format!("get data {:?}", err));
            }
        }

        if i == 255 {
            break;
        }
        i += 1;
    }
}

#[query(name = "getBlockNum")]
#[candid_method(query, rename = "getBlockNum")]
fn get_block_num() -> u64 {
    let mut i = 0;
    while i < 1000000 {
        let key = format!("{}", i);
        Bucket::insert_test(key.clone());
        i += 1;
        api::print(format!("i:{}, len:{}", i, key.len()));
    }
    i
}

#[query]
fn test_bucket() {
    let ret = Bucket::get_bucket();
    api::print(format!("offset : {}", ret.offset));
    for (k, v) in ret.assets {
        api::print(format!("k: {}, len: {}", k, v.len()));
    }
}

#[query]
fn test_upgrade() -> usize {
    let metadata = NFTINFO.with(|nft_info| {
        nft_info.borrow().clone()
    });

    let buf = Encode!(&metadata).unwrap();
    buf.len()
}
////////////////////////////////////////////////////////////
// test use all stable memory

//////边界值测试，最大读写的量

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}

#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    ic_cdk::export::candid::export_service!();
    std::print!("{}", __export_service());
}
