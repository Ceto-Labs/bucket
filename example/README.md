# rust_storage

- This program provides the use of buckets
  
```
cd ..
dfx start

dfx canister create example

dfx build example; dfx canister install example

dfx canister call example uploadData '(2047:nat)'

dfx canister call example checkData '(0, 255)'
```


- Store 8GB of data in stable and upgrade normally