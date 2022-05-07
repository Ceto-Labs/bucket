use bincode;
use ic_cdk::api::stable;
use ic_cdk::export::candid::{CandidType, Nat};
use ic_cdk::{trap};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;

static USER_DATA: &str = "user_data";
static mut THRESHOLD: u64 = 8589934592;
// 0 - 320 is used for offset. store key index()
static RESERVED_SPACE: u64 = 320 * MAX_PAGE_BYTE;
static MAX_PAGE_BYTE: u64 = 65536;

#[derive(CandidType, Deserialize, Debug, Clone)]
pub enum Error {
    InsufficientMemory,
    BlobSizeError,
    InvalidKey,
}

#[derive(CandidType, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Bucket {
    offset: u64,
    assets: HashMap<String, Vec<(u64, u64)>>, //(key,(offset len))
}

impl Default for Bucket {
    fn default() -> Self {
        Bucket {
            offset: RESERVED_SPACE,
            assets: HashMap::new(),
        }
    }
}

thread_local!(
    static BUCKET: RefCell<Bucket> = RefCell::new(Bucket::default());
);

impl Bucket {
    // ==================================================================================================
    // Auxiliary query  api
    // ==================================================================================================
    pub fn get_keys() -> Vec<String> {
        BUCKET.with(|bucket| {
            bucket.borrow().assets.clone().into_keys().collect()
        })
    }

    pub fn get_available_memory_size() -> Nat {
        BUCKET.with(|bucket| {
            let bucket = bucket.borrow();
            Nat::from(bucket._get_available_memory_size())
        })
    }

    // ==================================================================================================
    // danger  api
    // ==================================================================================================
    /// The index is deleted, but the data is still stored in stable
    pub fn del_key(key: String) {
        BUCKET.with(|bucket| {
            bucket.borrow_mut().assets.remove(&key);
        })
    }

    // ==================================================================================================
    // core api
    // ==================================================================================================

    pub fn put(key: String, value: Vec<u8>) -> Result<(), Error> {
        BUCKET.with(|bucket| {
            let mut bucket = bucket.borrow_mut();
            match bucket._get_field(value.len() as u64) {
                Ok(field) => {
                    match bucket.assets.get_mut(&key) {
                        None => {
                            bucket.assets.insert(key, vec![field.clone()]);
                        }
                        Some(pre_field) => {
                            pre_field.push(field.clone());
                        }
                    }
                    bucket._storage_data(field.0, value);

                    // todo check 索引大小，否则assert!
                    bucket._check_self_bytes_len();
                    Ok(())
                }
                Err(err) => {
                    return Err(err);
                }
            }
        })
    }

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
                        res.push(bucket._load_from_sm(f.clone()));
                    }
                    Ok(res)
                }
            }
        })
    }

    // ==================================================================================================
    // upgrade
    // ==================================================================================================
    /// NOTE:
    /// If you plan to store gigabytes of state and upgrade the code,
    /// Using put interface is a good option to consider
    pub fn pre_upgrade(buf: Vec<u8>) {
        match Bucket::put(USER_DATA.into(), buf) {
            Ok(..) => {}
            Err(err) => {
                trap(&*format!("pre_upgrade err {:?}", err))
            }
        };
        BUCKET.with(|bucket| {
            bucket.
                borrow_mut().
                _update_self_to_stable();
        });
    }

    pub fn post_upgrade() -> Vec<u8> {
        BUCKET.with(|bucket| {
            bucket.
                borrow_mut().
                _update_self_from_stable()
        });

        let buf = match Bucket::get(USER_DATA.into()) {
            Ok(vec) => { vec[0].clone() }
            Err(_err) => {
                trap(&*format!("bucket post upgrade err :{:?}", _err));
            }
        };

        // Reclaim the storage space occupied by user data  when upgrade
        BUCKET.with(|bucket| {
            bucket.borrow_mut().offset -= buf.len() as u64;
            bucket.borrow_mut().assets.remove(USER_DATA);
        });

        buf
    }

    // ==================================================================================================
    // private
    // ==================================================================================================
    fn _check_self_bytes_len(&self) {
        let bytes = bincode::serialize::<Bucket>(&self).unwrap();
        if bytes.len() as u64 >= RESERVED_SPACE {
            assert!(false)
        }
    }

    fn _update_self_to_stable(&mut self) {
        let bytes = bincode::serialize::<Bucket>(&self).unwrap();
        let bytes_len = bytes.len() as u64;
        self._grow_stable_memory_page(0);
        let len_bytes = Vec::from(bytes_len.to_be_bytes());

        self._storage_data(0, len_bytes.clone());
        self._storage_data(8, bytes);
    }
    fn _update_self_from_stable(&mut self) {
        let bucket_len_bytes: [u8; 8] = self._load_from_sm((0, 8))[..8]
            .try_into()
            .expect("update_self_from_stable : slice with incorrect length");
        let bucket_len = u64::from_be_bytes(bucket_len_bytes);

        let bucket_bytes = self._load_from_sm((8, bucket_len));
        let new_bucket: Bucket = bincode::deserialize(&bucket_bytes).unwrap();
        *self = new_bucket;
    }

    fn _load_from_sm(&self, field: (u64, u64)) -> Vec<u8> {
        let mut buf = [0].repeat(field.1 as usize);
        stable::stable64_read(field.0, &mut buf);
        buf.clone()
    }

    fn _get_field(&mut self, total_size: u64) -> Result<(u64, u64), Error> {
        match self._inspect_size(total_size.clone()) {
            Err(err) => Err(err),
            Ok(..) => {
                let field = (self.offset.clone(), total_size.clone());
                self._grow_stable_memory_page(total_size.clone());
                self.offset += total_size;
                Ok(field)
            }
        }
    }

    // check total_size
    fn _inspect_size(&self, total_size: u64) -> Result<(), Error> {
        if total_size <= self._get_available_memory_size() {
            Ok(())
        } else {
            Err(Error::InsufficientMemory)
        }
    }

    // When uploading, write data in the form of vals according to the assigned write_page
    fn _storage_data(&self, start: u64, data: Vec<u8>) {
        stable::stable64_write(start, data.as_slice());
    }

    // return available memory size can be allocated
    fn _get_available_memory_size(&self) -> u64 {
        unsafe {
            THRESHOLD - self.offset
        }
    }

    // grow SM memory pages of size "size"
    fn _grow_stable_memory_page(&self, size: u64) {
        if stable::stable64_size() == 0 {
            // Allocate reserved space
            match stable::stable64_grow(RESERVED_SPACE / MAX_PAGE_BYTE) {
                Ok(..) => {}
                Err(err) => trap(format!("{}", err).as_str())
            }
        }

        let available_mem = stable::stable64_size() * MAX_PAGE_BYTE - self.offset;
        if available_mem < size {
            let need_allo_size = size - available_mem;
            let grow_page = need_allo_size / MAX_PAGE_BYTE + 1;

            match stable::stable64_grow(grow_page) {
                Ok(..) => {}
                Err(err) => trap(format!("{}", err).as_str())
            }
        }
    }

    // ==================================================================================================
    // test code
    // ==================================================================================================
    // #[allow(dead_code)]
    // pub fn insert_test(key: String) {
    //     BUCKET.with(|bucket| {
    //         let mut bucket = bucket.borrow_mut();
    //
    //         let field = (1000000000000, 100000000000000);
    //         match bucket.assets.get_mut(&key) {
    //             None => {
    //                 bucket.assets.insert(key.clone(), vec![field.clone()]);
    //             }
    //             Some(pre_field) => {
    //                 pre_field.push(field.clone());
    //             }
    //         }
    //         bucket._check_self_bytes_len();
    //     });
    // }
}
