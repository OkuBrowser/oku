//! # showfile
//!
//! A simple API to show the location of a file in the local file manager (Explorer, Finder, etc.).
//! Supported platforms are Windows, macOS, Linux.
//!
//! ## Usage
//!
//! ```no_run
//! showfile::show_path_in_file_manager("C:\\Users\\Alice\\hello.txt");
//! showfile::show_path_in_file_manager("/Users/Bob/hello.txt");
//! showfile::show_uri_in_file_manager("file:///home/charlie/hello.txt");
//! ```
//!
//! # Feature Flags
//!
//! On Linux, D-Bus is used to invoke the file manager. The D-Bus crate in use can be selected with
//! one of these flags:
//!
//! - [`rustbus`](https://github.com/KillingSpark/rustbus) (default)
//! - [`zbus`](https://dbus2.github.io/zbus/)
//! - [`gio`](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/)
//!
//! One of these flags must be specified to build the project. These flags do nothing on Windows
//! and macOS. If only targeting those platforms, it can be left at the default.
//!
//! ## Details
//!
//! This crate is a simple wrapper around these system functions:
//!
//! - Windows: [`SHOpenFolderAndSelectItems`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shopenfolderandselectitems)
//! - macOS: [`NSWorkspace activateFileViewerSelectingURLs:`](https://developer.apple.com/documentation/appkit/nsworkspace/1524549-activatefileviewerselecting)
//! - Linux: [`org.freedesktop.FileManager1.ShowItems`](https://www.freedesktop.org/wiki/Specifications/file-manager-interface/)

use std::path::Path;

#[cfg(not(any(
    all(feature = "rustbus", not(feature = "zbus"), not(feature = "gio")),
    all(not(feature = "rustbus"), feature = "zbus", not(feature = "gio")),
    all(not(feature = "rustbus"), not(feature = "zbus"), feature = "gio")
)))]
compile_error!("only one of `rustbus`, `zbus`, or `gio` must be selected");

#[cfg_attr(target_os = "macos", link(name = "AppKit", kind = "framework"))]
extern "C" {}

#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
type id = *mut objc::runtime::Object;
#[cfg(target_os = "macos")]
#[allow(non_upper_case_globals)]
const nil: id = std::ptr::null_mut();

#[cfg(target_os = "macos")]
unsafe fn show_nsurl_in_file_manager(nsurl: id) {
    let ws: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let urls: id = msg_send![class!(NSArray), arrayWithObject:nsurl];
    let urls: id = msg_send![urls, autorelease];
    let _: () = msg_send![ws, activateFileViewerSelectingURLs:urls];
}

#[cfg(all(not(target_os = "macos"), not(windows), feature = "gio"))]
unsafe fn gdbus_show_uri_in_file_manager(uri: *const std::ffi::c_char) {
    use std::ptr::{null, null_mut};

    let bus = gio_sys::g_bus_get_sync(gio_sys::G_BUS_TYPE_SESSION, null_mut(), null_mut());
    if bus.is_null() {
        return;
    }
    let uris = [uri, null()];
    let args = glib_sys::g_variant_new(
        b"(^ass)\0".as_ptr() as *const _,
        uris.as_ptr(),
        b"\0".as_ptr(),
    );
    let ret = gio_sys::g_dbus_connection_call_sync(
        bus,
        b"org.freedesktop.FileManager1\0".as_ptr() as *const _,
        b"/org/freedesktop/FileManager1\0".as_ptr() as *const _,
        b"org.freedesktop.FileManager1\0".as_ptr() as *const _,
        b"ShowItems\0".as_ptr() as *const _,
        args,
        null(),
        0,
        -1,
        null_mut(),
        null_mut(),
    );
    if !ret.is_null() {
        glib_sys::g_variant_unref(ret);
    }
    gobject_sys::g_object_unref(bus as *mut _);
}

/// Tries to show `path` in a file manager.
///
/// The path shold be an absolute path. Support for relative paths is platform-specific and may
/// fail silently or cause the file manager to display an error message.
///
/// This function may do nothing at all depending on the current system. The result is
/// platform-specific if the path does not exist, is inaccessible, or if the file manager is
/// unavailable. The file manager may display an error message if a non-existent path is provided.
///
/// This function can block, so take care when calling from GUI programs. In those cases it should
/// be called on another thread, or called using your runtime's API to wrap blocking calls such as
/// [`tokio::task::spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
/// or [`gio::spawn_blocking`](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/fn.spawn_blocking.html).
pub fn show_path_in_file_manager(path: impl AsRef<Path>) {
    #[cfg(windows)]
    unsafe {
        use std::{
            borrow::Cow,
            path::{Component, Prefix},
        };
        use windows::{
            core::{Result, HSTRING},
            Win32::{System::Com::*, UI::Shell::*},
        };

        struct ComHandle(());
        impl ComHandle {
            fn new() -> Result<Self> {
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED)? };
                Ok(Self(()))
            }
        }
        impl Drop for ComHandle {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        std::thread_local! { static COM_HANDLE: Result<ComHandle> = ComHandle::new(); }
        COM_HANDLE.with(|r| r.as_ref().map(|_| ()).unwrap());

        let path = Cow::Borrowed(path.as_ref());

        // SHParseDisplayName seems to fail with UNC paths, so convert them back
        let mut components = path.components();
        let path = match components.next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::VerbatimUNC(server, share) => Cow::Owned(
                    Path::new("\\\\").join(Path::new(server).join(share).join(components)),
                ),
                Prefix::VerbatimDisk(disk) => {
                    let prefix = [disk, b':', b'\\'];
                    let prefix = std::ffi::OsStr::from_encoded_bytes_unchecked(&prefix);
                    Cow::Owned(Path::new(prefix).join(components))
                }
                Prefix::Verbatim(prefix) => {
                    Cow::Owned(Path::new("\\\\").join(Path::new(prefix).join(components)))
                }
                _ => path,
            },
            _ => path,
        };
        let mut idlist = std::ptr::null_mut();
        let res = SHParseDisplayName(
            &HSTRING::from(path.as_os_str()),
            None::<&IBindCtx>,
            &mut idlist,
            0,
            None,
        );
        if res.is_ok() && !idlist.is_null() {
            let _ = SHOpenFolderAndSelectItems(idlist, None, 0);
            CoTaskMemFree(Some(idlist as *const _));
        }
    }

    #[cfg(target_os = "macos")]
    unsafe {
        let path = path.as_ref().as_os_str().as_encoded_bytes();
        let s: id = msg_send![class!(NSString), alloc];
        let s: id = msg_send![
            s,
            initWithBytes:path.as_ptr()
            length:path.len()
            encoding:4 as id
        ];
        let s: id = msg_send![s, autorelease];
        let url: id = msg_send![class!(NSURL), fileURLWithPath:s];
        if url != nil {
            show_nsurl_in_file_manager(url);
        }
    }

    #[cfg(all(not(windows), not(target_os = "macos"), not(feature = "gio")))]
    {
        use std::path::Component;

        let path = path.as_ref();
        if path.is_relative() {
            return;
        }
        let mut uri = String::with_capacity(path.as_os_str().as_encoded_bytes().len() + 7);
        uri.push_str("file://");
        let mut components = path.components().peekable();
        if components.peek().is_none() {
            return;
        }
        while let Some(component) = components.next() {
            match component {
                Component::RootDir => uri.push('/'),
                Component::Prefix(_) => return,
                _ => {
                    let component = component.as_os_str().as_encoded_bytes();
                    uri.push_str(&urlencoding::encode_binary(component));
                    if components.peek().is_some() {
                        uri.push('/');
                    }
                }
            }
        }
        show_uri_in_file_manager(&uri);
    }

    #[cfg(all(not(windows), not(target_os = "macos"), feature = "gio"))]
    unsafe {
        let path = path.as_ref().as_os_str().as_encoded_bytes().to_vec();
        let path = std::ffi::CString::new(path).unwrap_or_else(|e| {
            let pos = e.nul_position();
            let mut uri = e.into_vec();
            uri.truncate(pos);
            std::ffi::CString::new(uri).unwrap()
        });
        let file = gio_sys::g_file_new_for_path(path.as_ptr());
        let uri = gio_sys::g_file_get_uri(file);
        if !uri.is_null() {
            if uri.read() != 0 {
                gdbus_show_uri_in_file_manager(uri);
            }
            glib_sys::g_free(uri as *mut _);
        }
        gobject_sys::g_object_unref(file as *mut _);
    }
}

/// Tries to show `uri` in a file manager.
///
/// URIs with the `file://` scheme should work on all platforms. On some platforms, the file
/// manager may be able to browse network URIs such as with the ftp://` or `smb://` schemes. The
/// file manager may fail silently or display an error message if given a non-supported URI scheme.
///
/// This function may do nothing at all depending on the current system. The result is
/// platform-specific if the path does not exist, is inaccessible, or if the file manager is
/// unavailable. The file manager may display an error message if a non-existent path is provided.
///
/// This function can block, so take care when calling from GUI programs. In those cases it should
/// be called on another thread, or called using your runtime's API to wrap blocking calls such as
/// [`tokio::task::spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
/// or [`gio::spawn_blocking`](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/fn.spawn_blocking.html).
pub fn show_uri_in_file_manager(uri: impl AsRef<str>) {
    #[cfg(windows)]
    show_path_in_file_manager(Path::new(uri.as_ref()));

    #[cfg(target_os = "macos")]
    unsafe {
        let uri = uri.as_ref();
        let s: id = msg_send![class!(NSString), alloc];
        let s: id = msg_send![
            s,
            initWithBytes:uri.as_ptr()
            length:uri.len()
            encoding:4 as id
        ];
        let s: id = msg_send![s, autorelease];
        let url: id = msg_send![class!(NSURL), URLWithString:s];
        if url != nil {
            show_nsurl_in_file_manager(url);
        }
    }

    #[cfg(all(not(target_os = "macos"), not(windows), feature = "rustbus"))]
    {
        if let Ok(mut bus) = rustbus::RpcConn::session_conn(rustbus::connection::Timeout::Infinite)
        {
            let uri = uri.as_ref();
            let mut msg = rustbus::MessageBuilder::new()
                .call("ShowItems")
                .on("/org/freedesktop/FileManager1")
                .with_interface("org.freedesktop.FileManager1")
                .at("org.freedesktop.FileManager1")
                .build();
            msg.body.push_param([uri].as_slice()).unwrap();
            msg.body.push_param("").unwrap();
            if let Ok(ctx) = bus.send_message(&mut msg) {
                let _ = ctx.write_all();
            }
            drop(bus);
        }
    }

    #[cfg(all(not(target_os = "macos"), not(windows), feature = "zbus"))]
    {
        let uri = uri.as_ref();
        if let Ok(bus) = zbus::blocking::Connection::session() {
            let _ = bus.call_method(
                Some("org.freedesktop.FileManager1"),
                "/org/freedesktop/FileManager1",
                Some("org.freedesktop.FileManager1"),
                "ShowItems",
                &([uri].as_slice(), ""),
            );
        }
    }

    #[cfg(all(not(target_os = "macos"), not(windows), feature = "gio"))]
    unsafe {
        let uri = uri.as_ref();
        let uri = std::ffi::CString::new(uri).unwrap_or_else(|e| {
            let pos = e.nul_position();
            let mut uri = e.into_vec();
            uri.truncate(pos);
            std::ffi::CString::new(uri).unwrap()
        });
        gdbus_show_uri_in_file_manager(uri.as_ptr());
    }
}
