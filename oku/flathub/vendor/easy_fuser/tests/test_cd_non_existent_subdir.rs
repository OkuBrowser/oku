#![cfg(any(feature = "serial", feature = "parallel"))]

#[cfg(all(feature = "parallel", not(feature = "serial")))]
use easy_fuser::fuse_parallel::prelude::*;
use easy_fuser::fuse_presets::DefaultFuseHandler;
use easy_fuser::fuse_presets::mirror_fs::*;
#[cfg(feature = "serial")]
use easy_fuser::fuse_serial::prelude::*;

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

struct MyFs {
    mirror_fs: MirrorFs,
    default_fs: DefaultFuseHandler<PathBuf>,
}

impl FuseHandler for MyFs {
    type TId = PathBuf;

    easy_fuser::delegate_fs! { mirror_fs, [ // readonly functions
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, open, readdir, readlink,
        ]
    }
    easy_fuser::delegate_fs! { mirror_fs, [ // readwrite functions
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr, symlink, unlink
        ]
    }

    easy_fuser::delegate_fs! {default_fs, [ bmap, forget, fsyncdir, getlk, ioctl, link, opendir, releasedir, setlk, statfs ]}
}

static MOUNT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_cd_non_existing_subdir_io_error() {
    let _lock = MOUNT_LOCK.lock().unwrap();
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();

    // Create temporary directories for mount point and source
    let mount_dir = TempDir::new().unwrap();
    let source_dir = TempDir::new().unwrap();

    let mntpoint = mount_dir.path().to_path_buf();
    let source_path = source_dir.path().to_path_buf();

    // Create a directory 'debug' in source_path
    let source_debug = source_path.join("debug");
    fs::create_dir(&source_debug).unwrap();

    let mntpoint_clone = mntpoint.clone();
    let source_path_clone = source_path.clone();
    let handle = std::thread::spawn(move || {
        let fs = MyFs {
            mirror_fs: MirrorFs::new(source_path_clone),
            default_fs: DefaultFuseHandler::new(),
        };
        mount(fs, &mntpoint_clone, &[], Some(4)).unwrap();
    });

    let mnt_debug = mntpoint.join("debug");
    let mut mounted = false;
    for _ in 0..100 {
        if mnt_debug.exists() {
            mounted = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(mounted, "Mount timed out");

    {
        // Run bash with LC_ALL=C to ensure English error messages
        let output = std::process::Command::new("bash")
            .env("LC_ALL", "C")
            .arg("-c")
            .arg(format!("cd {}/debug && sleep 2 && cd asd; ls", mntpoint.display()))
            .output()
            .expect("failed to execute bash command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("bash stdout: {}", stdout);
        println!("bash stderr: {}", stderr);
        println!("bash status: {:?}", output.status);

        // Check if we hit the Input/output error
        let has_io_error = stderr.contains("Input/output error");
        assert!(!has_io_error, "Detected Input/output error in stderr!");
    }

    // Unmount
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
    handle.join().unwrap();
}
