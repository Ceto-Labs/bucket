# rust_storage

- This program provides the use of buckets
  
```
cd ..
dfx start

dfx canister create example

dfx build example; dfx canister install example

dfx canister call example uploadData '(256:nat)'

dfx canister call example checkData '(0)'
```


- Store 8GB of data in stable and upgrade normally