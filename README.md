# rust_hello

- 启动dfx本地链

- dfx canister create storage

- dfx build storage; dfx canister install storage

- dfx canister call storage uploadData '(2047:nat)'

- dfx canister call storage checkData '(0:nat, 255:nat)'


- 4G 稳定内存情况下，可以存储数据2043MB

- 每个Key 46字节大小， 可以调用put 524315 左右

- 存储空间的理解   2G stable内存分配方式：{1MB数据索引 2043MB存储数据 3-4MB 存储系统元数据}
- 读
- 写 一次写入数据，2040MB左右
- 升级 
- 版本
- 数据量的限制
  一次可以写入最大值
  一次可以读取最大值
  索引占用空间数量


- ???
  12G 一个罐子，4G 罐子  8G stable