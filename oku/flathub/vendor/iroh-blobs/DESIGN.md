# Blob store trade offs

BLAKE3 hashed data and BLAKE3 verified streaming/bao are fully deterministic.

The iroh blobs protocol is implemented in this spirit. Even for complex requests like requests that involve multiple ranges or entire hash sequences, for every request, there is exactly one sequence of bytes that is the correct answer. And the requester will notice if data is incorrect after at most 16 KiB of data.

So why is designing a blob store for BLAKE3 hashed data so complex? What are the challenges?

## Terminology

The job of a blob store is to store two pieces of data per hash, the actual data itself and an `outboard` containing the BLAKE3 hash tree that connects each chunk of data to the root. Data and outboard are kept separate so that the data can be used as-is.

## In memory store

In the simplest case, you store data in memory. If you want to serve complete blobs, you just need a map from hash to data and outboard stored in a cheaply cloneable container like [bytes::Bytes]. Even if you want to also handle incomplete blobs, you use a map from hash and chunk number to (<= 1024 byte) data block, and that's it. Such a store is relatively easy to implement using a crate like [bao-tree] and a simple [BTreeMap] wrapped in a [Mutex].

But the whole point of a blob store - in particular for iroh-blobs, where we don't have an upper size limit for blobs - is to store *a lot* of data persistently. So while an in memory store implementation is included in iroh-blobs, an in memory store won't do for all use cases.

## Ok, fine. Let's use a database

We want a database that works on the various platforms we support for iroh, and that does not come with a giant dependency tree or non-rust dependencies. So our current choice is an embedded database called [redb]. But as we will see the exact choice of database does not matter that much.

A naive implementation of a block store using a relational or key value database would just duplicate the above approach. A map from hash and chunk number to a <= 1024 byte data block, except now in a persistent table.

But this approach has multiple problems. While it would work very well for small blobs, it would be very slow for large blobs. First of all, the database will have significant storage overhead for each block. Second, operating systems are very efficient in quickly accessing large files. Whenever you do disk IO on a computer, you will always go through the file system. So anything you add on top will only slow things down.

E.g. playing back videos from this database would be orders of magnitude slower than playing back from disk. And any interaction with the data would have to go through the iroh node, while with a file system we could directly use the file on disk.

## So, no database?

This is a perfectly valid approach when dealing exclusively with complete, large files. You just store the data file and the outboard file in the file system, using the hash as file name. But this approach has multiple downsides as well.

First of all, it is very inefficient if you deal with a large number of tiny blobs, like we frequently do when working with [iroh-docs] or [iroh-willow] documents. Just the file system metadata for a tiny file will vastly exceed the storage needed for the data itself.

Also, now we are very much dependent on the quirks of whatever file system our target operating system has. Many older file systems like FAT32 or EXT2 are notoriously bad in handling directories with millions of files. And we can't just limit ourselves to e.g. linux servers with modern file systems, since we also want to support mobile platforms and windows PCs.

And last but not least, creating a the metadata for a file is very expensive compared to writing a few bytes. We would be limited to a pathetically low download speed when bulk downloading millions of blobs, like for example an iroh collection containing the linux source code. For very small files embedded databases are [frequently faster](https://www.sqlite.org/fasterthanfs.html) than the file system.

There are additional problems when dealing with large incomplete files. Enough to fill a book. More on that later.

## What about a hybrid approach

It seems that databases are good for small blobs, and the file system works great for large blobs. So what about a hybrid approach where small blobs are stored entirely inline in a database, but large blobs are stored as files in the file system? This is complex, but provides very good performance both for millions of small blobs and for a few giant blobs.

We won't ever have a directory with millions of files, since tiny files never make it to the file system in the first place. And we won't need to read giant files through the bottleneck of a database, so access to large files will be as fast and as concurrent as the hardware allows.

This is exactly the approach that we currently take in the iroh-blobs file based store. For both data files and outboards, there is a configurable threshold below which the data will be stored inline in the database. The configuration can be useful for debugging and testing, e.g. you can force all data on disk by setting both thresholds to 0. But the defaults of 16 KiB for inline outboard and data size make a lot of sense and will rarely have to be changed in production.

While this is the most complex approach, there is really no alternative if you want good performance both for a large number of tiny blobs and a small number of giant blobs.

# The hybrid blob store in detail

### Sizes

A complication with the hybrid approach is that when we have a hash to get data for, we do not know its size. So we can not immediately decide whether it should go into a file or the database. Even once we request the data from a remote peer, we don't know the exact size. A remote node could lie about the size, e.g. tell us that the blob has an absurdly large size. As we get validated chunks the uncertainty about the size decreases. After the first validated chunk of data, the uncertainty is only a factor of 2, and as we receive the last chunk we know the exact size.

This is not the end of the world, but it does mean that we need to keep the data for a hash in some weird indeterminate state before we have enough data to decide if it will be stored in memory or on disk.

### Metadata

We want to have the ability to quickly list all blobs or determine if we have data for a blob. Small blobs live completely in the database, so for these this is not a problem. But even for large blobs we want to keep a bit of metadata in the database as well, such as whether they are complete and if not if we at least have a verified size. But we also don't want to keep more than this in the metadata database, so we don't have to update it frequently as new data comes in. An individual database update is very fast, but syncing the update to disk is extremely slow no matter how small the update is. So the secret to fast download speeds is to not touch the metadata database at all if possible.

### The metadata table

The metadata table is a mapping from a BLAKE3 hash to an entry state. The rough entry state can be either `complete` or `partial`.

#### Complete entry state

Data can either be `inline` in the metadata db, `owned` in a canonical location in the file system (in this case we don't have to store the path since it can be computed from the hash), or `external` in a number of user owned paths that have to be explicitly stored. In the `owned` or `external` case we store the size in the metadata, since getting it from a file is an io operation with non-zero cost.

An outboard can either be `inline` in the metadata db, `owned` in a canonical location in the file system, or `not needed` in case the data is less large than a chunk group size. Any data <= 16 KiB will have an outboard of size 0 which we don't bother to store at all. Outboards are never in user owned paths, so we never have to store a path for them.

#### Partial entry state

For a partial entry, there are just two possible states. Once we have the last chunk we have a verified size, so we store it here. If we do not yet have the last chunks, we do not have a size.

### Inline data and inline outboard table

The inline data table is kept separate from the metadata table since it will tend to be much bigger. Mixing inline data with metadata would make iterating over metadata entries slower. For the same reason, the outboard table is separate. You could argue that the inline outboard and data should be combined in a single table, but I decided against it to keep a simple table structure. Also, for the default settings if the data is inline, no outboard will be needed at all. Likewise when an outboard is needed, the data won't be inline.

### Tags table

The tags table is just a simple CRUD table that is completely independent of the other tables. It is just kept together with the other tables for simplicity.


todo: move this section!

The way this is currently implemented is as follows: as we start downloading a new blob from a remote, the information about this blob is kept fully in memory. The database is not touched at all, which means that the data would be gone after a crash. On the plus side, no matter what lies a remote node tells us, they will not create much work for us.

As soon as we know that the data is going to be larger than the inlining threshold, we create an entry in the db marking the data as present but incomplete, and create files for the data and outboard. There is also an additional file which contains the most precisely known size.

As soon as the blob becomes complete, an entry is created that marks the data as complete. At this point the inlining rules will be applied. Data that is below the inline threshold will be moved into the metadata database, likewise with the outboard.

## Blob lifecycle

There are two fundamentally different ways how data can be added to a blob store. 

### Adding local files by name

If we add data locally, e.g. from a file, we have the data but don't know the hash. We have to compute the complete outboard, the root hash, and then atomically move the file into the store under the root hash. Depending on the size of the file, data and outboard will end up in the file system or in the database. If there was partial data there before, we can just replace it with the new complete data and outboard. Once the data is stored under the hash, neither the data nor the outboard will be changed.

### Syncing remote blobs by hash

If we sync data from a remote node, we do know the hash but don't have the data. In case the blob is small, we can request and atomically write the data, so we have a similar situation as above. But as soon as the data is larger than a chunk group size (16 KiB), we will have a situation where the data has to be written incrementally. This is much more complex than adding local files. We now have to keep track of which chunks of the blob we have locally, and which chunks of the blob we can *prove* to have locally (not the same thing if you have a chunk group size > 1).

### Blob deletion

On creation, blobs are tagged with a temporary tag that prevents them from being deleted for as long as the process lives. They can then be tagged with a persistent tag that prevents them from being deleted even after a restart. And last but not least, large groups of blobs can be protected from deletion in bulk by putting a sequence of hashes into a blob and tagging that blob as a hash sequence.

We also provide a way to explicitly delete blobs by hash, but that is meant to be used only in case of an emergency. You have some data that you want **gone** no matter how dire the consequences are.

Deletion is always *complete*. There is currently no mechanism to protect or delete ranges of a file. This makes a number of things easier. A chunk of a blob can only go from not present (all zeroes) to present (whatever the correct data is for the chunk), not the other way. And this means that all changes due to syncing from a remote source commute, which makes dealing with concurrent downloads from multiple sources much easier.

Deletion runs in the background, and conceptually you should think of it as always being on. Currently there is a small delay between a blob being untagged and it being deleted, but in the future blobs will be immediately deleted as soon as they are no longer tagged.

### Incomplete files and the file system

File system APIs are not very rich. They don't provide any transactional semantics or even ordering guarantees. And what little guarantees they provide is different from platform to platform.

This is not much of a problem when dealing with *complete* files. You have a file for the data and a file for the outboard, and that's it. You never write to them, and reliably *reading* ranges from files is something even the most ancient file system like FAT32 or EXT2 can reliably handle.

But it gets much more complex when dealing with *incomplete* files. E.g. you start downloading a large file from a remote node, and add chunks one by one (or in the case of n0 flavoured bao in chunk groups of 16 KiB).

The challenge now is to write to the file system in such a way that the data remains consistent even when the write operation is rudely interrupted by the process being killed or the entire computer crashing. Losing a bit of data that has been written immediately before the crash is tolerable. But we don't *ever* want to have a situation after a crash where we *think* we have some data, but actually we are missing something about the data or the outboard that is needed to verify the data.

BLAKE3 is very fast, so for small to medium files we can just validate the blob on node startup. For every chunk of data in the file, compute the hash and check it against the data in the outboard. This functionality exists in the [bao-tree] crate. It will give you a bitfield for every chunk that you have locally. But not even BLAKE3 is fast enough to do this for giant files like ML models or disk images. For such files it would take many *minutes* at startup to compute the validity bitfield, so we need a way to persist it.

And that comes with its own rabbit hole of problems. As it turns out, [files are hard](https://danluu.com/file-consistency/). In particular writing to files.

## Files are hard

As discussed above, we want to keep track of data and outboard in the file system for large blobs. Let's say we use three files, `<hash>.data`, `<hash>.outboard` and `<hash>.bitfield`. As we receive chunks of verified data via [bao], we write the hashes to the outboard file, the data to the data file, and then update the bitfield file with the newly valid chunks. But we need to do the update in *exactly this order*, otherwise we might have a situation where the bitfield file has been updated before the data and outboard file, meaning that the bitfield file is inconsistent with the other files. The only way to fix this would be to do a full validation, which is what we wanted to avoid.

We can of course enforce order by calling `fsync` on the data and outboard file *before* updating the bitfield file, after each chunk update. But that is horribly expensive no matter how fast your computer is. A sync to file system will take on the order of milliseconds, even on SATA SSDs. So clearly that is not an option. If you sync after each chunk, and a sync takes 5ms, your write speed is now limited to 200 kilobytes per second. Even if you sync after each 16 KiB chunk group, your download speed is now limited to 3.2 megabytes per second.

But if we don't do the sync, there is no guarantee whatsoever in what order or if at all the data makes it to disk. So in case of a crash our bitfield could be inconsistent with the data, so we would be better off not persisting the bitfield at all.

Most operating systems have platform dependent ways to finely control syncing for memory mapped files. E.g. on POSIX systems there is the `msync` API and on windows the similar `FlushViewOfFile` API. But first of all we are not confident that memory mapping will work consistently on all platforms we are targeting. And also, this API is not as useful as it might seem.

While it provides fine grained control over syncing individual ranges of memory mapped files, it does not provide a way to guarantee that *no* sync happens unless you explicitly call it. So if you map the three files in memory and then update them, even if you don't call `msync` at all there is no guarantee in which order the data makes it to disk. The operating system page cache does not know about files or about write order, it will persist pages to disk in whatever order is most convenient for its internal data structures.

So what's the solution? We can write to the data and outboard file in any order and without any sync calls as often as we want. These changes will be invisible after a crash provided that the bitmap file is not updated.