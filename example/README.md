# rust_storage

- This program provides the use of buckets
  
```
dfx start

dfx canister create storage

dfx build storage; dfx canister install storage

dfx canister call storage uploadData '(2047:nat)'

dfx canister call storage checkData '(0, 255)'
```


- Store 8GB of data in stable and upgrade normally