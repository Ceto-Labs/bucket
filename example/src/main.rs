use ic_cdk::api;
use ic_cdk::export::candid::{candid_method, CandidType, Nat, Decode, Encode, Deserialize};
use num_traits::ToPrimitive;
use std::cell::RefCell;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};
// use serde::{Deserialize, Serialize};

use bucket;

#[derive(CandidType, Deserialize, Clone)]
struct NftInfo {
    read_offset: u64,
    write_offset: u64,
    nft_data: Vec<u8>,
    read_write: u64,
}

impl Default for NftInfo {
    fn default() -> Self {
        NftInfo {
            read_offset: 0,
            nft_data: vec![],
            write_offset: 0,
            read_write: 0,
        }
    }
}

static USER_DATA: &'static str = "user_data";
static WRITE_BLOCK_SIZE: u64 = 64 * 1024;
static WITE_COUNT: u64 = 64;

thread_local!(
    /* stable */   static NFTINFO: RefCell<NftInfo> = RefCell::new(NftInfo::default());
);

#[init]
#[candid_method(init)]
fn init() {
    NFTINFO.with(|nftinfo| {
        let data = [0].repeat(104857600);
        nftinfo.borrow_mut().nft_data.extend(data.iter())
    });
}

#[ic_cdk_macros::query]
fn greet(name: String) -> String {
    format!("Hello yy bb 5 00 r335566, {}!", name)
}

// NOTE:
// If you plan to store gigabytes of state and upgrade the code,
// Using stable memory as the main storage is a good option to consider
#[pre_upgrade]
fn pre_upgrade() {
    let _nftinfo = NFTINFO.with(|nftinfo| {
        nftinfo.borrow().clone()
    });

    // let bytes = bincode::serialize::<NftInfo>(&_nftinfo).unwrap();

    let bytes = Encode!(&_nftinfo).unwrap();
    if let Err(_err) = bucket::put(&USER_DATA.to_string(), bytes) {
        assert!(false)
    }
    bucket::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    bucket::post_upgrade();

    let bytes = bucket::get(&USER_DATA.to_string()).unwrap();
    NFTINFO.with(|nftinfo| {
        // *nftinfo.borrow_mut() = bincode::deserialize(&bytes).unwrap()
        *nftinfo.borrow_mut() = Decode!(&bytes, NftInfo).unwrap();
    });

    bucket::del(&USER_DATA.to_string())
}

////////////////////////////////////////////////////////////
#[update(name = "testKV")]
#[candid_method(update, rename = "testKV")]
fn test_kv() {
    let key_count = Nat::from(256u32);
    upload_data(key_count.clone());

    let mut i = 0;
    while i < key_count.0.to_i32().unwrap() {
        check_data(Nat::from(i));
        i += 1;
    }
}

#[update(name = "uploadData")]
#[candid_method(update, rename = "uploadData")]
fn upload_data(num: Nat) {
    let num = num.0.to_u32().unwrap();
    let mut i = 0;
    while i < num {
        let mut j = 0;
        while j < WITE_COUNT {
            let data: Vec<u8> = vec![i as u8; WRITE_BLOCK_SIZE as usize];
            let name = format!("{}.png", i as u8);
            let ret = bucket::append(&name, data);
            match ret {
                Ok(..) => {}
                Err(err) => {
                    api::print(format!("upload data err:{:?}", err));
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
    let ret = bucket::put(&name, data);
    match ret {
        Ok(..) => {}
        Err(err) => api::print(format!("upload data  err:{:?}", err)),
    }
}

#[update(name = "checkData")]
#[candid_method(update, rename = "checkData")]
fn check_data(index: Nat) {
    let name = format!("{}.png", index.0.to_u32().unwrap() as u8);

    if bucket::get_size(&name) != WITE_COUNT * WRITE_BLOCK_SIZE {
        api::print(format!("check data, key size:{},{} KB", bucket::get_size(&name), WITE_COUNT * WRITE_BLOCK_SIZE/1024));
        assert!(false)
    }

    api::print(name.clone());
    let ret = bucket::get(&name);
    match ret {
        Ok(data) => {
            if data.len() as u64 != WITE_COUNT * WRITE_BLOCK_SIZE {
                assert!(false)
            }
            for v in &data {
                if v.clone() != index.0.to_u32().unwrap() as u8 {
                    api::print(format!("vec:{:?}", v));
                    assert!(false);
                }
            }
        }
        Err(err) => {
            api::print(format!("get data {:?}", err));
        }
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

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}

#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    ic_cdk::export::candid::export_service!();
    std::print!("{}", __export_service());
}
