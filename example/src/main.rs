use ic_cdk::api;
use ic_cdk::export::candid::{candid_method, CandidType, Nat, Decode, Encode, Deserialize};
use num_traits::ToPrimitive;
use std::cell::RefCell;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};
// use serde::{Deserialize, Serialize};

use kv;

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
            read_offset: 1111,
            nft_data: vec![],
            write_offset: 2222,
            read_write: 3333,
        }
    }
}

static USER_DATA: &'static str = "user_data";
static WRITE_BLOCK_SIZE: u64 = 64 * 1024;
static WRITE_COUNT: u64 = 16;
static KEY_COUNT: u8 = 128;

thread_local!(
    /* stable */   static NFTINFO: RefCell<NftInfo> = RefCell::new(NftInfo::default());
);

#[init]
#[candid_method(init)]
fn init() {
    NFTINFO.with(|nftinfo| {
        let data = vec!(5; 1024 * 1024);
        nftinfo.borrow_mut().nft_data.extend(data.iter())
    });
}

#[ic_cdk_macros::query]
fn greet(name: String) -> String {
    format!("Hello chbi,4455 {}!", name)
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
    if let Err(_err) = kv::put(&USER_DATA.to_string(), bytes) {
        assert!(false)
    }
    kv::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    kv::post_upgrade();

    let bytes = kv::get(&USER_DATA.to_string()).unwrap();
    NFTINFO.with(|nftinfo| {
        // *nftinfo.borrow_mut() = bincode::deserialize(&bytes).unwrap()
        *nftinfo.borrow_mut() = Decode!(&bytes, NftInfo).unwrap();
    });

    kv::del(&USER_DATA.to_string())
}

////////////////////////////////////////////////////////////
// test api
////////////////////////////////////////////////////////////

#[update(name = "getIndexSpace")]
#[candid_method(query, rename = "getIndexSpace")]
fn get_upgrade_left_space() -> u64 {
    kv::get_index_space()
}

#[update(name = "getUtilization")]
#[candid_method(query, rename = "getUtilization")]
fn get_utilization() -> f64 {
    kv::get_utilization()
}

#[update(name = "checkBitMap")]
#[candid_method(query, rename = "checkBitMap")]
fn check_bit_map() -> String {
    let old_map = kv::get_bit_map();
    let key = "test_bit_map_key".to_string();
    let ret = kv::put(&key, vec!(2; 1024 * 782));
    match ret {
        Ok(..) => {}
        Err(err) => {
            api::print(format!("upload data err:{:?}", err));
            assert!(false)
        }
    }

    kv::del(&key);

    let new_map = kv::get_bit_map();
    assert!(old_map.eq(&new_map));

    "checkBitMap pass".to_string()
}


#[update(name = "testKV")]
#[candid_method(update, rename = "testKV")]
fn test_kv() -> String {
    // check data
    upload_data();

    let mut i = 0;
    while i <= KEY_COUNT {
        check_single_data(Nat::from(i));
        i += 1;
    }

    "testKV pass".to_string()
}

#[update(name = "uploadData")]
#[candid_method(update, rename = "uploadData")]
fn upload_data() -> String {
    let mut i = 0;
    while i <= KEY_COUNT {
        let mut j = 0;
        while j < WRITE_COUNT {
            let data: Vec<u8> = vec![i as u8; WRITE_BLOCK_SIZE as usize];
            let name = format!("{}.png", i as u8);
            let ret = kv::append(&name, data);
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
    "uploadData pass".to_string()
}

#[update(name = "uploadBigData")]
#[candid_method(update, rename = "uploadBigData")]
fn upload_big_data(num: u32) {
    let data: Vec<u8> = vec![1 as u8; (num * 1024 * 1024) as usize];

    let name = format!("{}.png", 0);
    let ret = kv::put(&name, data);
    match ret {
        Ok(..) => {}
        Err(err) => api::print(format!("upload data  err:{:?}", err)),
    }
}

#[update(name = "checkData")]
#[candid_method(update, rename = "checkData")]
fn check_data() -> String {
    let mut i = 0;
    while i <= KEY_COUNT {
        check_single_data(Nat::from(i));
        i += 1;
    }

    "checkData pass".to_string()
}

#[update(name = "checkDel")]
#[candid_method(update, rename = "checkDel")]
fn check_del() -> String {
    let mut i = 0;
    while i <= KEY_COUNT {
        let name = format!("{}.png", i as u8);
        kv::del(&name);
        i += 1;
    }

    "checkDel pass".to_string()
}


fn check_single_data(index: Nat) {
    let name = format!("{}.png", index.0.to_u32().unwrap() as u8);

    if kv::get_size(&name) != WRITE_COUNT * WRITE_BLOCK_SIZE {
        api::print(format!("check data, key size:{} KB,{} KB", kv::get_size(&name) / 1024, WRITE_COUNT * WRITE_BLOCK_SIZE / 1024));
        assert!(false)
    }

    let ret = kv::get(&name);
    match ret {
        Ok(data) => {
            if data.len() as u64 != WRITE_COUNT * WRITE_BLOCK_SIZE {
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
fn check_upgrade() -> NftInfo {
    NFTINFO.with(|nft_info| {
        nft_info.borrow().clone()
    })
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
