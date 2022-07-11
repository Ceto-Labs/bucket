use crate::types::*;
use crate::stable::*;
use std::cell::RefCell;

//罐子最大能分配到的空间8GB
static THRESHOLD: u64 = 8589934592;

//从0自己开始预留20MB,用于存储元数据
pub static RESERVED_SPACE: u64 = RESERVED_PAGE * MAX_PAGE_BYTE / 512 * KV_BLOCK_SIZE;

//ic stable每个页大小
static MAX_PAGE_BYTE: u64 = 65536;

//kv中每个BLOCK大小
pub static KV_BLOCK_SIZE: u64 = 512;

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
        (RESERVED_SPACE + block_number * KV_BLOCK_SIZE + block_offset) as u64
    }

    pub(crate) fn bit_map(&self) -> Vec<u8> {
        self.bit_map.clone()
    }

    pub(crate) fn new_blocks(&mut self, count: u64) -> Result<Vec<u64>, KvError> {
        //1 从已经分配到的系统stable-block获取
        let mut tem_count = count;
        let mut blocks = vec![];
        for (index, byte) in self.bit_map.iter_mut().enumerate() {
            let mut i = 0;
            while i < 8 && tem_count > 0 {
                if (*byte & 1u8 << i) == 0 {
                    *byte |= 1u8 << i;

                    let new_block_number = index * 8 + i;
                    blocks.push(new_block_number as u64);
                    tem_count -= 1;
                }
                i += 1;
            }

            if tem_count == 0 {
                break;
            }
        }

        if tem_count == 0 {
            for block in &blocks {
                let position = self.get_position(block.clone() as u64, KV_BLOCK_SIZE);
                if position > self.stable_blocks_count * MAX_PAGE_BYTE {
                    //2 触发从系统中获取stable空间;
                    match self._grow_stable_memory_page(1) {
                        Ok(..) => {}
                        Err(err) => { return Err(err); }
                    }
                }
            }

            return Ok(blocks);
        }

        return Err(KvError::Other("not enough space".into()));
    }

    pub(crate) fn free_blocks(&mut self, blocks: Vec<u64>) {
        for block_number in blocks {
            let index = (block_number / 8) as usize;
            assert!(index < self.bit_map.len());

            let byte = self.bit_map.get_mut(index).unwrap();
            *byte &= !(1 << (block_number % 8)) as u8;
        }
    }

    // return available memory size can be allocated
    pub(crate) fn get_available_memory_size(&self) -> u64 {
        //stable剩余空间 + kv中剩余空间
        let mut left_block_size = 0;

        for byte in &self.bit_map {
            let mut i = 0;
            while i < 8 {
                if (byte & 1 << i) == 0 {
                    left_block_size += 1;
                }
                i += 1;
            }
        }

        THRESHOLD - self.stable_blocks_count * MAX_PAGE_BYTE + left_block_size * KV_BLOCK_SIZE
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
        if byte & 1 << 2 == 0 {
            println!("{}", 1)
        }
    }
}