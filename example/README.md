# rust_storage

- This program provides the use of buckets
  
```
cd ..
dfx start

dfx canister create example

dfx build example; dfx canister install example

dfx canister call example uploadData

dfx canister call example  checkBitMap

// change code ; then upgrade
dfx build example; dfx canister install example --mode upgrade

dfx canister call example  checkData

dfx canister call example checkDel 

```


- Store 8GB of data in stable and upgrade normally