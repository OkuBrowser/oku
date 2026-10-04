Those are examples of toys program that could be added or not in the future to demonstrate usage of easy_fuser

# encrypt_fs
A fuse system that will store files and directories in a plain folder.
## Dependencies:
- ring crate
- bincode crate
- base64 crate
## Specifications:
### File encryption
- Files are encrypted using AES_256_GCM in blocks of 16384 bytes, with nonce + tag at the beginning of each block.
  This allow to still use size+seek on both read and write without reading the entire file
- the correct key should be verified against the tag+nonce before overwriting a block
### File name encryption
- File names should be hashed/encrypted, then converted to base64 then truncated to length of 255
- A special file should contain the correspondance between the plain and hash filenames.
  This file won't appear in mounted filesystem and will be encrypted and deserialized using bincode crate
- This file should be updated on rename, unlink and any other operations that create a new file (create, mkdir)

# sql_fs
Should be a simple example of combining a sqlite database to store inodes/path correspondance, while storing files inside a folder
## source dir structure
a sqlite database and a unique folder containing each file named after a unique id
## sql tables
- Table1: `[KEY file_id, EXTERN KEY parent_id, file_name]`
- Table2: `[KEY parent_id, EXTERN KEY file_id]`

# extended_metadata_fs
A filesystem that will show a modifiable structure metadata file for each file to store more metadata
## Specifications
- Additional metadata could be stored inside any format (like sql)
- the mounted filesystem will show a `.<file_name>.metadata` for each file
- opening this file will open a copy inside /tmp
- releasing the file will trigger a validation of this file before saving the additional metdata in the database