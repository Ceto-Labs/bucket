#V2 features
- Reserve 20MB space to store bucket index
- The maximum available stable storage space is 8GB
- GB level data storage, using the put interface
- If you want to upgrade the MB level data in the user's canister without losing it, you can use pre_upgrade storage, post_upgrade recovery
- To store data in the key/value format
- you can delete the key and release the stable storage space corresponding to the value (the freed space is not returned to the ic network) for use by other keys.
- The space allocated from the ic network is managed according to the mode of the file system to manage the disk, so the space will be divided into blocks of 512bytes size

- logical architecture
![](./logical_architecture.jpg "logical architecture")

- file system layout 
![logical architecture](./implement.jpg)


# How to use