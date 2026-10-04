// This test uses delegate_fs! (sync) which is incompatible with async-only builds.
// Async mode is covered by tests/async_test.rs instead.
#![cfg(any(feature = "serial", feature = "parallel"))]

#[cfg(all(feature = "parallel", not(feature = "serial")))]
use easy_fuser::fuse_parallel::prelude::*;
#[cfg(feature = "serial")]
use easy_fuser::fuse_serial::prelude::*;

use easy_fuser::fuse_presets::{DefaultFuseHandler, mirror_fs::*};

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// In theory, this test shouldn't work, but for an unknown reason it works
///
/// However, when trying this kind of mount in a terminal, it hangs
use easy_fuser_macro::delegate_fs;

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
fn test_mirror_fs_recursion() {
    // Create source and mount directories
    let source_path = PathBuf::from("/tmp/easy_fuser_recursion_fs_source");
    let mntpoint = source_path.join("mount");

    // Ensure the directories are clean
    let _ = fs::remove_dir_all(&source_path);
    fs::create_dir_all(&source_path).unwrap();
    fs::create_dir_all(&mntpoint).unwrap();

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
        std::thread::sleep(Duration::from_millis(500));
    }
    assert!(mounted, "Mount timed out");

    // Construct the deep recursive path
    let deep_path = mntpoint
        .join("easy_fuser_recursion_fs_source")
        .join("mount")
        .join("easy_fuser_recursion_fs_source")
        .join("mount")
        .join("easy_fuser_recursion_fs_source")
        .join("mount");

    // Try to list the contents of the deep recursive path
    let start_time = Instant::now();
    let output = Command::new("ls")
        .arg(&deep_path)
        .output()
        .expect("Failed to execute ls command");

    let elapsed_time = start_time.elapsed();

    // Check if the command completed within 5 seconds
    if elapsed_time >= Duration::from_secs(5) {
        panic!(
            "Test failed: 'ls' command took 5 seconds or more, indicating a potential infinite recursion."
        );
    }

    // Check the output of the 'ls' command
    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("ls output: {}", output_str);
    println!("ls error: {}", String::from_utf8_lossy(&output.stderr));

    // The exact output might depend on how your filesystem handles this case,
    // but it should not show an infinitely recursive structure
    assert!(
        !output_str.contains("easy_fuser_recursion_fs_source"),
        "The output suggests an infinitely recursive structure"
    );

    println!("Test passed: No infinite recursion detected.");

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
    #[cfg(not(any(target_os = "freebsd", target_os = "macos")))]
    handle.join().unwrap();
    #[cfg(target_os = "freebsd")]
    // TODO: explore why error: no such file or directory happens there
    let _ = handle.join();
    #[cfg(target_os = "macos")] // TODO: why handle.join is blocking ?
    drop(handle);
    drop(mntpoint);
}
