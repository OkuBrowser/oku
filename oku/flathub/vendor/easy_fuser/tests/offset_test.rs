// This test uses delegate_fs! (sync) which is incompatible with async-only builds.
// Async mode is covered by tests/async_test.rs instead.
#![cfg(any(feature = "serial", feature = "parallel"))]

#[cfg(all(feature = "parallel", not(feature = "serial")))]
use easy_fuser::fuse_parallel::prelude::*;
#[cfg(feature = "serial")]
use easy_fuser::fuse_serial::prelude::*;

use easy_fuser::fuse_presets::{DefaultFuseHandler, mirror_fs::*};

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Duration;
use tempfile::TempDir;

use easy_fuser_macro::delegate_fs;
use std::path::PathBuf;

struct MyFs {
    mirror_fs: MirrorFs,
    default_fs: DefaultFuseHandler<PathBuf>,
}

impl FuseHandler for MyFs {
    type TId = PathBuf;

    delegate_fs! { mirror_fs, [
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, open, readdir, readlink,
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr, symlink, unlink
    ]}

    delegate_fs! { default_fs, [ bmap, forget, fsyncdir, getlk, ioctl, link, opendir, releasedir, setlk, statfs ] }
}

#[test]
fn test_mirror_fs_file_offsets() {
    // Create temporary directories for mount point and source
    let mount_dir = TempDir::new().unwrap();
    let source_dir = TempDir::new().unwrap();

    let mntpoint = mount_dir.path().to_path_buf();
    let source_path = source_dir.path().to_path_buf();

    // We won't use spawn_mount because it MirrorFs doesn't implement Send in serial mode
    let mntpoint_clone = mntpoint.clone();
    let source_path_clone = source_path.clone();
    let sentinel = source_path.join("sentinel.txt");
    fs::write(&sentinel, "").unwrap();

    let handle = std::thread::spawn(move || {
        let fs = MyFs {
            mirror_fs: MirrorFs::new(source_path_clone),
            default_fs: DefaultFuseHandler::new(),
        };
        mount(fs, &mntpoint_clone, &[], Some(4)).unwrap();
    });

    let mnt_sentinel = mntpoint.join("sentinel.txt");
    let mut mounted = false;
    for _ in 0..100 {
        if mnt_sentinel.exists() {
            mounted = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(mounted, "Mount timed out");

    // Contrary to using spawn, which will force unmount even if resource is busy,
    // Here we must clean it before
    {
        // Create a test file
        let test_file = mntpoint.join("offset_test.txt");
        let mut file = File::create(&test_file).unwrap();

        // Write initial content
        file.write_all(b"Hello, World!").unwrap();
        file.sync_all().unwrap();

        // Test reading from different offsets
        let mut file = File::open(&test_file).unwrap();
        let mut buffer = [0u8; 5];

        // Read from the beginning
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut buffer).unwrap();
        assert_eq!(&buffer, b"Hello");

        // Read from an offset
        file.seek(SeekFrom::Start(7)).unwrap();
        file.read_exact(&mut buffer).unwrap();
        assert_eq!(&buffer, b"World");

        // Test writing at different offsets
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&test_file)
            .unwrap();

        // Write at an offset
        file.seek(SeekFrom::Start(7)).unwrap();
        file.write_all(b"Rust!").unwrap();
        file.sync_all().unwrap();

        // Verify the write
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        // "Hello, World!" contains one character more
        assert_eq!(content, "Hello, Rust!!");

        // Test seeking relative to current position
        file.seek(SeekFrom::Start(0)).unwrap();
        file.seek(SeekFrom::Current(7)).unwrap();
        file.read_exact(&mut buffer).unwrap();
        assert_eq!(&buffer, b"Rust!");

        // Test seeking relative to the end
        file.seek(SeekFrom::End(-1)).unwrap();
        file.read_exact(&mut buffer[0..1]).unwrap();
        assert_eq!(buffer[0], b'!');

        // Test writing beyond the end of the file
        file.seek(SeekFrom::End(0)).unwrap();
        file.write_all(b" Extended").unwrap();
        file.sync_all().unwrap();

        // Verify the extended content
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut extended_content = String::new();
        file.read_to_string(&mut extended_content).unwrap();
        assert_eq!(extended_content, "Hello, Rust!! Extended");
    }

    eprintln!("Unmounting filesystem...");
    let mut unmounted = false;
    for cmd_name in &["fusermount3", "fusermount", "umount"] {
        let mut cmd = std::process::Command::new(cmd_name);
        if cmd_name == &"umount" {
            cmd.arg(&mntpoint);
        } else {
            cmd.arg("-u").arg(&mntpoint);
        }
        if let Ok(status) = cmd.status() {
            if status.success() {
                eprintln!("Unmounted successfully using {}", cmd_name);
                unmounted = true;
                break;
            }
        }
    }
    assert!(
        unmounted,
        "Failed to unmount using fusermount3, fusermount, or umount"
    );
    // To allow integration tests to not fail if unmount fails, comment the assert above and uncomment the block below:
    // if !unmounted {
    //     eprintln!("Warning: Failed to unmount using fusermount3, fusermount, or umount");
    // }
    #[cfg(not(any(target_os = "freebsd")))]
    handle.join().unwrap();
    #[cfg(target_os = "freebsd")]
    // TODO: explore why error: no such file or directory happens there
    let _ = handle.join();
    drop(mntpoint);
}
