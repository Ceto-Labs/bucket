use crate::types::*;
use crate::layout::*;
use crate::stable::*;
use bincode;
use std::cell::RefCell;
use crate::{layout, stable};
// use serde::{Deserialize, Serialize};

mod kv {
    use super::*;
    use std::cmp::min;

    thread_local!(static KV: RefCell<Kv> = RefCell::new(Kv::default()));

    pub fn with<T, F: FnOnce(&Kv) -> T>(f: F) -> T {
        KV.with(|kv| f(&kv.borrow()))
    }

    pub fn with_mut<T, F: FnOnce(&mut Kv) -> T>(f: F) -> T {
        KV.with(|kv| f(&mut kv.borrow_mut()))
    }

    impl Kv {
        pub fn check_upgrade(&self) -> bool {
            let len = _get_upgrade_data().len();
            if 8 + len as u64 >= layout::RESERVED_SPACE {
                return false;
            }
            return true;
        }

        pub fn get_index_space(&self) -> u64 {
            let used = _get_upgrade_data();
            used.len() as u64
        }

        pub fn get_keys(&self) -> Vec<String> {
            self.kv_set.clone().into_keys().collect()
        }

        /// The index is deleted, but the data is still stored in stable
        pub fn del(&mut self, key: &String) {
            let value = self.kv_set.remove(key);

            if let Some((blocks, _)) = value {
                layout::with_mut(|layout| {
                    layout.free_blocks(blocks);
                });
            }
        }

        pub fn append(&mut self, key: &String, value: Vec<u8>) -> Result<(), KvError> {
            let mut value = value;

            match self.kv_set.get_mut(key) {
                Some((blocks, position)) => {

                    //剩余空间可以存储全部新来的数据
                    let stable_position = layout::with_mut(|layout| {
                        layout.get_position(blocks[blocks.len() - 1], *position)
                    });

                    let storage_data_len = min((layout::KV_BLOCK_SIZE - *position) as usize, value.len());
                    storage_data(stable_position, value[0..storage_data_len].to_vec());
                    value = value[(layout::KV_BLOCK_SIZE - *position) as usize..].to_owned();
                    *position += value.len() as u64;

                    if layout::KV_BLOCK_SIZE > *position {
                        return Ok(());
                    }
                }
                None => return self.put(key, value)
            }

            let (new_blocks, new_position) = match self._insert_data(value) {
                Ok((new_blocks, new_position)) => {
                    (new_blocks, new_position)
                }
                Err(err) => { return Err(err); }
            };

            let (blocks, position) = self.kv_set.get_mut(key).unwrap();
            blocks.extend(new_blocks.clone());
            *position = new_position;
            Ok(())
        }

        pub fn put(&mut self, key: &String, value: Vec<u8>) -> Result<(), KvError> {
            if let Some(..) = self.kv_set.get(key) {
                return Err(KvError::Other("key exist, please use func append() or del key first".into()));
            }

            match self._insert_data(value) {
                Ok((blocks, position)) => {
                    self.kv_set.insert(key.clone(), (blocks, position));
                    Ok(())
                }
                Err(err) => { Err(err) }
            }
        }

        pub fn get(&self, key: &String) -> Result<Vec<u8>, KvError> {
            match self.kv_set.get(key) {
                Some((blocks, position)) => {
                    let mut data = vec![];
                    for (index, b) in blocks.iter().enumerate() {
                        let mut data_len = KV_BLOCK_SIZE;
                        if index == blocks.len() - 1 {
                            data_len = position.clone();
                        }
                        let stable_position = layout::with_mut(|layout| {
                            layout.get_position(b.clone(), 0)
                        });

                        data.extend(load_from_stable(stable_position, data_len as usize))
                    }
                    Ok(data)
                }
                None => Err(KvError::InvalidKey)
            }
        }

        pub fn get_content_size(&self, key: &String) -> u64 {
            match self.kv_set.get(key) {
                None => { 0u64 }
                Some((blocks, position)) => {
                    (blocks.len() - 1) as u64 * KV_BLOCK_SIZE + position.clone()
                }
            }
        }

        pub fn get_utilization(&self) -> f64 {
            let mut block_count = 0;
            let mut unused_space = 0;
            for (_key, (blocks, position)) in &self.kv_set {
                block_count += blocks.len();
                unused_space += KV_BLOCK_SIZE - *position;
            }

            (1f64 - (unused_space as f64) / (block_count as u64 * KV_BLOCK_SIZE) as f64) * 100f64
        }
        // ==================================================================================================
        // private
        // ==================================================================================================
        // 经常检查，会消耗大量cycle

        fn _insert_data(&mut self, value: Vec<u8>) -> Result<(Vec<u64>, u64), KvError> {
            let mut value = value;
            let mut position: usize = layout::KV_BLOCK_SIZE as usize;

            let need_block_count = if value.len() as u64 % layout::KV_BLOCK_SIZE == 0 {
                value.len() as u64 / layout::KV_BLOCK_SIZE
            } else {
                value.len() as u64 / layout::KV_BLOCK_SIZE + 1
            };

            let blocks = match layout::with_mut(|layout| layout.new_blocks(need_block_count)) {
                Ok(new_blocks) => { new_blocks }
                Err(err) => return Err(err)
            };

            for block in &blocks {
                //分配新块
                let stable_position = layout::with_mut(|layout| {
                    layout.get_position(block.clone(), 0u64)
                });

                let storage_data_len = min((layout::KV_BLOCK_SIZE) as usize, value.len());

                storage_data(stable_position, value[0..storage_data_len].to_vec());
                value = value[storage_data_len..].to_vec();
                position = storage_data_len;
            }
            assert_eq!(value.len(), 0);

            Ok((blocks, position as u64))
        }
    }
}

// ==================================================================================================
// core api
// ==================================================================================================
pub fn get_size(key: &String) -> u64 {
    kv::with(|kv| kv.get_content_size(key))
}

pub fn get(key: &String) -> Result<Vec<u8>, KvError> {
    kv::with(|kv| kv.get(key))
}

pub fn put(key: &String, value: Vec<u8>) -> Result<(), KvError> {
    kv::with_mut(|kv| kv.put(key, value))
}

pub fn append(key: &String, value: Vec<u8>) -> Result<(), KvError> {
    kv::with_mut(|kv| kv.append(key, value))
}

pub fn del(key: &String) {
    kv::with_mut(|kv| kv.del(key))
}

pub fn get_keys() -> Vec<String> {
    kv::with(|kv| kv.get_keys())
}

//查询
pub fn get_available_space_size() -> u64 {
    layout::with(|layout| layout.get_available_memory_size())
}

pub fn get_bit_map() -> Vec<u8> {
    layout::with(|layout| layout.bit_map())
}

//
pub fn check_upgrade() -> bool {
    kv::with(|kv| kv.check_upgrade())
}

pub fn get_index_space() -> u64 {
    kv::with(|kv| kv.get_index_space())
}

pub fn get_utilization() -> f64 {
    kv::with(|kv| kv.get_utilization())
}

// ==================================================================================================
// upgrade
// ==================================================================================================
// NOTE:
// If you plan to store gigabytes of state and upgrade the code,
// Using put interface is a good option to consider
pub fn pre_upgrade() {
    _update_self_to_stable();
}

pub fn post_upgrade() {
    _update_self_from_stable();
}

fn _update_self_to_stable() {
    let bytes = _get_upgrade_data();
    let bytes_len = bytes.len() as u64;

    if 8 + bytes_len >= layout::RESERVED_SPACE {
        assert!(false);
    }

    if let Err(_err) = stable::stable_grow_memory_page(0) {
        assert!(false)
    }

    let len_bytes = Vec::from(bytes_len.to_be_bytes());
    storage_data(0, len_bytes.clone());
    storage_data(8, bytes);
}

fn _update_self_from_stable() {
    let bucket_len_bytes: [u8; 8] = load_from_stable(0, 8)[..8]
        .try_into()
        .expect("update_self_from_stable : slice with incorrect length");

    let bucket_len = u64::from_be_bytes(bucket_len_bytes);

    let bucket_bytes = load_from_stable(8, bucket_len as usize);
    let new_stable_struct: StableStruct = bincode::deserialize(&bucket_bytes).unwrap();

    kv::with_mut(|kv| *kv = new_stable_struct.kv);
    layout::with_mut(|layout| *layout = new_stable_struct.layout);
}

fn _get_upgrade_data() -> Vec<u8> {
    let stable_struct = StableStruct {
        layout: layout::with(|layout| { layout.clone() }),
        kv: kv::with(|kv| { kv.clone() }),
    };

    bincode::serialize::<StableStruct>(&stable_struct).unwrap()
}


#[test]
fn test_put() {}

#[test]
fn test_get() {}

#[test]
fn test_put_get() {}

#[test]
fn get_available_size() {
    get_available_space_size();
}

