use bincode;
use ic_cdk::api::stable;
use ic_cdk::export::candid::{CandidType, Nat};
use ic_cdk::{api, trap};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;

static USER_DATA: &str = "user_data";
// MAX_PAGE_SIZE = 8 GB(total size of stable memory currently) / 64 KB(each page size = 64 KB)
static mut THRESHOLD: u64 = 8589934592;

//存储系统数据
static RESERVED_SPACE_SYSTEM: u64 = 0;

// 0 - 31 is used for offset. store key index()
static RESERVED_SPACE: u64 = 320 * MAX_PAGE_BYTE;
// static MAX_QUERY_SIZE: u64 = 3144728;
// static MAX_PAGE_NUMBER: u64 = 131072;
static MAX_PAGE_BYTE: u64 = 65536;

#[derive(CandidType, Deserialize, Debug, Clone)]
pub enum Error {
    InsufficientMemory,
    BlobSizeError,
    InvalidKey,
}

#[derive(CandidType, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Bucket {
    pub(crate) upgradable: bool,
    pub(crate) offset: u64,
    pub(crate) assets: HashMap<String, Vec<(u64, u64)>>, //(key,(offset len))
}

impl Default for Bucket {
    fn default() -> Self {
        Bucket {
            upgradable: true,
            offset: RESERVED_SPACE,
            assets: HashMap::new(),
        }
    }
}

thread_local!(
    static BUCKET: RefCell<Bucket> = RefCell::new(Bucket::default());
);

impl Bucket {
    pub fn get_bucket() -> Bucket {
        BUCKET.with( | bucket| {
            bucket.borrow().clone()
        })
    }

    pub fn init(upgrade: bool) {
        BUCKET.with(|bucket| {
            bucket.borrow_mut().upgradable = upgrade;
        })
    }
    fn check_self_bytes_len(&self) {
        let bytes = bincode::serialize::<Bucket>(&self).unwrap();
        if bytes.len() as u64 >= RESERVED_SPACE {
            assert!(false)
        }
    }
    pub fn update_self_to_stable(&self) {
        let bytes = bincode::serialize::<Bucket>(&self).unwrap();
        let bytes_len = bytes.len() as u64;
        Bucket::_grow_stable_memory_page(&self, 0);
        let len_bytes = Vec::from(bytes_len.to_be_bytes());

        Bucket::_storage_data(0, len_bytes.clone());
        Bucket::_storage_data(8, bytes);
    }
    pub fn update_self_from_stable(&mut self) {
        let bucket_len_bytes: [u8; 8] = Bucket::_load_from_sm((0, 8))[..8]
            .try_into()
            .expect("update_self_from_stable : slice with incorrect length");
        let bucket_len = u64::from_be_bytes(bucket_len_bytes);

        let bucket_bytes = Bucket::_load_from_sm((8, bucket_len));
        let new_bucket: Bucket = bincode::deserialize(&bucket_bytes).unwrap();
        *self = new_bucket;
    }

    pub fn get_keys() -> Vec<String> {
        BUCKET.with(|bucket| {
            bucket.borrow().assets.clone().into_keys().collect()
        })
    }

    // 删除索引，但是数据还存储在stable中
    pub fn del_key(key: String) {
        BUCKET.with(|bucket| {
            bucket.borrow_mut().assets.remove(&key);
        })
    }

    #[allow(dead_code)]
    pub fn set(key: String, value: Vec<u8>) -> Result<(), Error> {
        // if  value.len() > 100*1024*1024{
        //     return Err(Error::BlobSizeError)
        // }
        BUCKET.with(|bucket| {
            let mut bucket = bucket.borrow_mut();
            match Bucket::_get_field(&mut bucket, value.len() as u64) {
                Ok(field) => {
                    bucket.assets.insert(key, vec![field.clone()]);
                    Bucket::_storage_data(field.0, value);

                    // todo check 索引大小，否则assert!
                    bucket.check_self_bytes_len();
                    // let bytes = bincode::serialize::<Bucket>(&bucket).unwrap();
                    // api::print(format!("----bytes.len: {}", bytes.len()));

                    // if bytes.len() as u64 >= RESERVED_SPACE {
                    //     assert!(false)
                    // }
                    Ok(())
                }
                Err(err) => {
                    return Err(err);
                }
            }
        })
    }

    pub fn put(key: String, value: Vec<u8>) -> Result<(), Error> {
        BUCKET.with(|bucket| {
            let mut bucket = bucket.borrow_mut();
            match Bucket::_get_field(&mut bucket, value.len() as u64) {
                Ok(field) => {
                    match bucket.assets.get_mut(&key) {
                        None => {
                            bucket.assets.insert(key, vec![field.clone()]);
                        }
                        Some(pre_field) => {
                            pre_field.push(field.clone());
                        }
                    }
                    Bucket::_storage_data(field.0, value);

                    // todo check 索引大小，否则assert!
                    bucket.check_self_bytes_len();
                    // let bytes = bincode::serialize::<Bucket>(&bucket).unwrap();
                    // api::print(format!("----bytes.len: {}", bytes.len()));

                    // if bytes.len() as u64 >= RESERVED_SPACE {
                    //     assert!(false)
                    // }
                    Ok(())
                }
                Err(err) => {
                    return Err(err);
                }
            }
        })
    }

    #[allow(dead_code)]
    pub fn insert_test(key: String) {
        BUCKET.with(|bucket| {
            let mut bucket = bucket.borrow_mut();

            let field = (1000000000000, 100000000000000);
            match bucket.assets.get_mut(&key) {
                None => {
                    bucket.assets.insert(key.clone(), vec![field.clone()]);
                }
                Some(pre_field) => {
                    pre_field.push(field.clone());
                }
            }
            bucket.check_self_bytes_len();
        });
    }
    // todo 读取数据空间限制
    pub fn get(key: String) -> Result<Vec<Vec<u8>>, Error> {
        BUCKET.with(|bucket| {
            let bucket = bucket.borrow();
            match bucket.assets.get(&key) {
                None => {
                    return Err(Error::InvalidKey);
                }
                Some(field) => {
                    let mut res = vec![];
                    for f in field.iter() {
                        res.push(Bucket::_load_from_sm(f.clone()));
                    }
                    Ok(res)
                }
            }
        })
    }

    #[allow(dead_code)]
    pub fn get_available_memory_size() -> Nat {
        BUCKET.with(|bucket| {
            let bucket = bucket.borrow();
            Nat::from(Bucket::_get_available_memory_size(&bucket))
        })
    }

    // return entries
    pub fn pre_upgrade(buf: Vec<u8>) {
        // todo 限制buf长度

        match Bucket::put(USER_DATA.into(), buf) {
            Ok(..) => {}
            Err(err) => {
                trap(&*format!("pre_upgrade err {:?}", err))
            }
        };
        let bucket = BUCKET.with(|bucket| bucket.borrow().clone());
        Bucket::update_self_to_stable(&bucket);
    }

    pub fn post_upgrade() -> Vec<u8> {
        BUCKET.with(|bucket| {
            let mut bucket = bucket.borrow_mut();
            Bucket::update_self_from_stable(&mut bucket)
        });

        let buf = match Bucket::get(USER_DATA.into()) {
            Ok(vec) => { vec[0].clone() }
            Err(_err) => {
                trap(&*format!("bucket post upgrade err :{:?}", _err));
            }
        };

        // 收回用户数据锁占用的存储空间
        BUCKET.with(|bucket| {
            bucket.borrow_mut().offset -= buf.len() as u64;
            bucket.borrow_mut().assets.remove(USER_DATA);
        });

        buf
    }
    //
    fn _load_from_sm(field: (u64, u64)) -> Vec<u8> {
        let mut buf = [0].repeat(field.1 as usize);
        stable::stable64_read(field.0, &mut buf);
        buf.clone()
    }

    fn _get_field(bucket: &mut Bucket, total_size: u64) -> Result<(u64, u64), Error> {
        match Bucket::_inspect_size(bucket, total_size.clone()) {
            Err(err) => Err(err),
            Ok(..) => {
                let field = (bucket.offset.clone(), total_size.clone());
                Bucket::_grow_stable_memory_page(bucket, total_size.clone());
                bucket.offset += total_size;
                Ok(field)
            }
        }
    }

    // check total_size
    fn _inspect_size(bucket: &Bucket, total_size: u64) -> Result<(), Error> {
        if total_size <= Bucket::_get_available_memory_size(bucket) {
            Ok(())
        } else {
            Err(Error::InsufficientMemory)
        }
    }

    // upload时根据分配好的write_page以vals的形式写入数据
    // When uploading, write data in the form of vals according to the assigned write_page
    fn _storage_data(start: u64, data: Vec<u8>) {
        stable::stable64_write(start, data.as_slice());
    }

    // return available memory size can be allocated
    fn _get_available_memory_size(bucket: &Bucket) -> u64 {
        unsafe {
            if bucket.upgradable {
                assert!(THRESHOLD / 2 >= bucket.offset);
                THRESHOLD / 2 - bucket.offset - RESERVED_SPACE_SYSTEM
            } else {
                THRESHOLD - bucket.offset - RESERVED_SPACE_SYSTEM
            }
        }
    }

    // grow SM memory pages of size "size"
    fn _grow_stable_memory_page(bucket: &Bucket, size: u64) {
        if bucket.offset == RESERVED_SPACE {
            // 预留的空间分配好
            let ret = stable::stable64_grow(RESERVED_SPACE / MAX_PAGE_BYTE + 1);
        }

        let available_mem = stable::stable64_size() * MAX_PAGE_BYTE - bucket.offset;
        if available_mem < size {
            let need_allo_size = size - available_mem;
            let grow_page = need_allo_size / MAX_PAGE_BYTE + 1;

            // todo 处理返回值
            let ret = stable::stable64_grow(grow_page);
        }
    }
}
