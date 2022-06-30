use crate::types::*;
use crate::stable::*;
use std::cell::RefCell;

static USER_DATA: &str = "user_data";
static mut THRESHOLD: u64 = 8589934592;
// 0 - 320 is used for offset. store key index()
static RESERVED_SPACE: u64 = 320 * MAX_PAGE_BYTE;
static MAX_PAGE_BYTE: u64 = 65536;
static KV_BLOCK_SIZE: u64 = 512;
static RESERVED_BYTES_SIZE: usize = (RESERVED_SPACE / KV_BLOCK_SIZE / 8) as usize;//预留512个块


mod layout {
    use super::*;

    thread_local!(
        static LAYOUT: RefCell<Layout> = RefCell::new(Layout::default());
    );

    pub fn with<T, F: FnOnce(&Layout) -> T>(f: F) -> T {
        LAYOUT.with(|layout| f(&layout.borrow()))
    }

    pub fn with_mut<T, F: FnOnce(&mut Layout) -> T>(f: F) -> T {
        // LAYOUT.with(|layout| f(&mut layout.borrow_mut()));
        LAYOUT.with(|layout| f(&mut layout.borrow_mut()))
    }


    // impl Default for Layout {
    //     fn default() -> Self {
    //         Layout {
    //             stable_blocks_count: 0,
    //             bit_map: vec![]
    //         }
    //     }
    // }

    impl Layout {
        fn get_position(&self, block_number: u64) -> u64 {
            block_number * KV_BLOCK_SIZE
        }

        fn new_block(&mut self) -> Result<u64, KvError> {
            //1 从已经分配到的系统stable-block获取
            let mut found = false;

            loop {
                for (index, byte) in self.bit_map[RESERVED_BYTES_SIZE..].iter().enumerate() {
                    let mut i = 0;
                    while i < 8 {
                        if (byte & 1 << i) == 0 {
                            found = true;
                            break;
                        }

                        i += 1;
                    }

                    if found {
                        return Ok(((RESERVED_BYTES_SIZE + index) * 8 + i) as u64);
                    }
                }

                //2 触发从系统中获取stable-block; 返回到第一步，否则报错返回
                match self._grow_stable_memory_page(1) {
                    Ok(..) => {}
                    Err(err) => { return Err(err); }
                }
            }
        }

        fn free_block(&mut self, key: String, block_number: u64) -> bool {
            false
        }

        fn free_key(&mut self, key: String) -> bool { false }


        // return available memory size can be allocated
        fn get_available_memory_size(&self) -> u64 {
            //stable剩余空间 + kv中剩余空间
            let mut left_block_size = 0;

            for byte in &self.bit_map[RESERVED_BYTES_SIZE..] {
                let mut i = 0;
                while i < 8 {
                    if (byte & 1 << i) == 0 {
                        left_block_size += 1;
                    }
                    i += 1;
                }
            }

            unsafe {
                THRESHOLD - self.stable_blocks_count * MAX_PAGE_BYTE + left_block_size * MAX_PAGE_BYTE
            }
        }

        ////////////////////////////////////////////////////////////////////////////////////////////
        /// private fn
        ////////////////////////////////////////////////////////////////////////////////////////////

        // grow SM memory pages of size "size"
        fn _grow_stable_memory_page(&mut self, grow_page: u64) -> Result<(), KvError> {
            match stable_grow_memory_page(grow_page) {
                Ok(..) => {
                    self.stable_blocks_count += grow_page;
                    let len: usize = (grow_page * MAX_PAGE_BYTE / 512 / 8) as usize;
                    self.bit_map.extend_from_slice(vec!(0; len).as_slice());
                    Ok(())
                }
                Err(err) => Err(err)
            }
        }
    }
}

mod test {
    #[test]
    fn bit_test() {
        let byte = 2u8;
        if byte & 1 << 2 {
            println!("{}", 1)
        }
    }
}