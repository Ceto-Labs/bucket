use crate::types::*;
use crate::stable::*;
use std::cell::RefCell;
use ic_cdk::api;

//罐子最大能分配到的空间8GB
static THRESHOLD: u64 = 8589934592;

//从0自己开始预留20MB,用于存储元数据
static RESERVED_SPACE: u64 = RESERVED_PAGE * MAX_PAGE_BYTE;

//ic stable每个页大小
static MAX_PAGE_BYTE: u64 = 65536;

//kv中每个BLOCK大小
pub static KV_BLOCK_SIZE: u64 = 65536;

//用于存储索引（元数据）的空间，不做分配
static RESERVED_BYTES_SIZE: usize = (RESERVED_SPACE / KV_BLOCK_SIZE / 8) as usize;

pub fn with<T, F: FnOnce(&Layout) -> T>(f: F) -> T {
    LAYOUT.with(|layout| f(&layout.borrow()))
}

pub fn with_mut<T, F: FnOnce(&mut Layout) -> T>(f: F) -> T {
    LAYOUT.with(|layout| f(&mut layout.borrow_mut()))
}

thread_local!(static LAYOUT: RefCell<Layout> = RefCell::new(Layout::default()););

impl Default for Layout {
    fn default() -> Self {
        Layout {
            stable_blocks_count: RESERVED_PAGE,
            bit_map: vec!(0; (THRESHOLD - RESERVED_SPACE) as usize / KV_BLOCK_SIZE as usize / 8),
            kv_block_size: KV_BLOCK_SIZE,
        }
    }
}

impl Layout {
    pub(crate) fn get_position(&self, block_number: u64, block_offset: u64) -> u64 {
        block_number * KV_BLOCK_SIZE + block_offset
    }

    pub(crate) fn new_block(&mut self) -> Result<u64, KvError> {
        //1 从已经分配到的系统stable-block获取
        let mut found = false;

        for (index, byte) in self.bit_map.iter_mut().enumerate() {
            let mut i = 0;
            while i < 8 {
                if (*byte & 1u8 << i) == 0 {
                    *byte |= 1u8 << i;
                    api::print(format!("index:{}, byte:{}", index, *byte));
                    found = true;
                    break;
                }

                i += 1;
            }

            if found {
                api::print(format!("diff:{},{}", ((RESERVED_BYTES_SIZE + index) * 8 + i) as u64 * KV_BLOCK_SIZE, self.stable_blocks_count * MAX_PAGE_BYTE));

                if ((RESERVED_BYTES_SIZE + index) * 8 + i) as u64 * KV_BLOCK_SIZE >=
                    self.stable_blocks_count * MAX_PAGE_BYTE {
                    //2 触发从系统中获取stable-block; 返回到第一步，否则报错返回
                    match self._grow_stable_memory_page(1) {
                        Ok(..) => {}
                        Err(err) => { return Err(err); }
                    }
                }
                return Ok(((RESERVED_BYTES_SIZE + index) * 8 + i) as u64);
            }
        }

        return Err(KvError::Other("not enough space".into()));
    }

    pub(crate) fn free_block(&mut self, block_number: u64) {
        let index = if block_number % 8 == 0 {
            (block_number / 8) as usize
        } else {
            (block_number / 8 + 1) as usize
        };
        assert!(index < self.bit_map.len());
        let byte = self.bit_map.get_mut(index).unwrap();
        *byte &= !(1 << (block_number % 8)) as u8;
    }

    // return available memory size can be allocated
    pub(crate) fn get_available_memory_size(&self) -> u64 {
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

        THRESHOLD - self.stable_blocks_count * MAX_PAGE_BYTE + left_block_size * MAX_PAGE_BYTE
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    /// private fn
    ////////////////////////////////////////////////////////////////////////////////////////////

    // grow SM memory pages of size "size"
    fn _grow_stable_memory_page(&mut self, grow_page: u64) -> Result<(), KvError> {
        match stable_grow_memory_page(grow_page) {
            Ok(..) => {
                self.stable_blocks_count += grow_page;
                Ok(())
            }
            Err(err) => Err(err)
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