// This test uses delegate_fs! (sync) which is incompatible with async-only builds.
// Async mode is covered by tests/async_test.rs instead.
#![cfg(any(feature = "serial", feature = "parallel"))]

#[cfg(all(feature = "parallel", not(feature = "serial")))]
use easy_fuser::fuse_parallel::prelude::*;
use easy_fuser::fuse_presets::DefaultFuseHandler;
use easy_fuser::fuse_presets::mirror_fs::*;
#[cfg(feature = "serial")]
use easy_fuser::fuse_serial::prelude::*;

use easy_fuser_macro::delegate_fs;

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

struct MyFs {
    mirror_fs: MirrorFs,
    default_fs: DefaultFuseHandler<PathBuf>,
}

impl FuseHandler for MyFs {
    type TId = PathBuf;

    delegate_fs! { mirror_fs, [ // readonly functions
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, open, readdir, readlink,
        ]
    }
    delegate_fs! { mirror_fs, [ // readwrite functions
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr, symlink, unlink
        ]
    }

    delegate_fs! {default_fs, [ bmap, forget, fsyncdir, getlk, ioctl, link, opendir, releasedir, setlk, statfs ]}
}

struct MyFsReadOnly {
    mirror_fs: MirrorFsReadOnly,
    default_fs: DefaultFuseHandler<PathBuf>,
}

impl FuseHandler for MyFsReadOnly {
    type TId = PathBuf;

    delegate_fs! { mirror_fs, [
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, open, readdir, readlink
    ]}

    delegate_fs! { default_fs, [
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr, symlink, unlink,
        bmap, forget, fsyncdir, getlk, ioctl, link, opendir, releasedir, setlk, statfs
    ]}
}

static MOUNT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_mirror_fs_operations() {
    let _lock = MOUNT_LOCK.lock().unwrap();
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

    {
        // Create a file and check its existence
        let test_file = mntpoint.join("test_file.txt");
        File::create(&test_file).unwrap();
        assert!(test_file.exists());

        // Create a directory and check its existence
        let test_dir = mntpoint.join("test_dir");
        fs::create_dir(&test_dir).unwrap();
        assert!(test_dir.exists());

        // Write to a file and retrieve its content
        let content = "Hello, MirrorFs!";
        fs::write(&test_file, content).unwrap();
        let read_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, read_content);

        // Remove a file
        fs::remove_file(&test_file).unwrap();
        assert!(!test_file.exists());

        // Remove a directory
        fs::remove_dir(&test_dir).unwrap();
        assert!(!test_dir.exists());

        // Print and modify file attributes
        let new_file = mntpoint.join("attribute_test.txt");
        File::create(&new_file).unwrap();

        let metadata = fs::metadata(&new_file).unwrap();
        println!("Initial permissions: {:o}", metadata.permissions().mode());
        println!("Initial owner: {}:{}", metadata.uid(), metadata.gid());

        // Change permissions
        let new_permissions = 0o644;
        fs::set_permissions(&new_file, fs::Permissions::from_mode(new_permissions)).unwrap();

        let updated_metadata = fs::metadata(&new_file).unwrap();
        println!(
            "Updated permissions: {:o}",
            updated_metadata.permissions().mode()
        );
        assert_eq!(
            updated_metadata.permissions().mode() & 0o777,
            new_permissions
        );

        // Truncate a file
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&new_file)
            .unwrap();
        file.write_all(b"Initial content").unwrap();
        drop(file);

        let initial_len = fs::metadata(&new_file).unwrap().len();
        assert_eq!(initial_len, 15);

        let truncate_len = 5;
        let file = fs::OpenOptions::new().write(true).open(&new_file).unwrap();
        file.set_len(truncate_len).unwrap();
        drop(file);

        let truncated_len = fs::metadata(&new_file).unwrap().len();
        assert_eq!(truncated_len, truncate_len);
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
    eprintln!("Joining thread...");
    #[cfg(not(any(target_os = "freebsd")))]
    handle.join().unwrap();
    #[cfg(target_os = "freebsd")]
    // TODO: explore why error: no such file or directory happens there
    let _ = handle.join();
    eprintln!("Thread joined successfully!");
    drop(mntpoint);
}

#[test]
fn test_mirror_fs_readonly_operations() {
    let _lock = MOUNT_LOCK.lock().unwrap();
    // Create temporary directories for mount point and source
    let mount_dir = TempDir::new().unwrap();
    let source_dir = TempDir::new().unwrap();

    let mntpoint = mount_dir.path().to_path_buf();
    let source_path = source_dir.path().to_path_buf();

    // Create a file in the source directory
    let test_file_name = "readonly_test.txt";
    let source_file = source_path.join(test_file_name);
    fs::write(&source_file, "Read-only test content").unwrap();

    let mntpoint_clone = mntpoint.clone();
    let source_path_clone = source_path.clone();
    let handle = std::thread::spawn(move || {
        let fs = MyFsReadOnly {
            mirror_fs: MirrorFsReadOnly::new(source_path_clone),
            default_fs: DefaultFuseHandler::new(),
        };
        mount(fs, &mntpoint_clone, &[], Some(4)).unwrap();
    });

    let mnt_file = mntpoint.join(test_file_name);
    let mut mounted = false;
    for _ in 0..100 {
        if mnt_file.exists() {
            mounted = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(mounted, "Mount timed out");

    {
        // Read file via mount point should succeed
        let mnt_file = mntpoint.join(test_file_name);
        assert!(mnt_file.exists());
        let content = fs::read_to_string(&mnt_file).unwrap();
        assert_eq!(content, "Read-only test content");

        // Write/Create file via mount point should fail with ENOSYS
        let new_file = mntpoint.join("new_file.txt");
        let write_result = fs::write(&new_file, "This should fail");
        assert!(write_result.is_err());
        let err = write_result.unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::ENOSYS));
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
    eprintln!("Joining thread...");
    #[cfg(not(any(target_os = "freebsd")))]
    handle.join().unwrap();
    #[cfg(target_os = "freebsd")]
    let _ = handle.join();
    eprintln!("Thread joined successfully!");
    drop(mntpoint);
}
