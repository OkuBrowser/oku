use crate::{WebsiteData, WebsiteDataManager, WebsiteDataTypes, prelude::*};

use glib::translate::*;

use std::{boxed::Box as Box_, ptr};

impl WebsiteDataManager {
    #[doc(alias = "webkit_website_data_manager_clear")]
    pub fn clear<P: FnOnce(Result<(), glib::Error>) + Send + 'static>(
        &self,
        types: WebsiteDataTypes,
        timespan: glib::TimeSpan,
        cancellable: Option<&impl IsA<gio::Cancellable>>,
        callback: P,
    ) {
        let user_data: Box_<P> = Box_::new(callback);
        unsafe extern "C" fn clear_trampoline<
            P: FnOnce(Result<(), glib::Error>) + Send + 'static,
        >(
            _source_object: *mut glib::gobject_ffi::GObject,
            res: *mut gio::ffi::GAsyncResult,
            user_data: glib::ffi::gpointer,
        ) {
            unsafe {
                let mut error = ptr::null_mut();
                let _ = ffi::webkit_website_data_manager_clear_finish(
                    _source_object as *mut _,
                    res,
                    &mut error,
                );
                let result = if error.is_null() {
                    Ok(())
                } else {
                    Err(from_glib_full(error))
                };
                let callback: Box_<P> = Box_::from_raw(user_data as *mut _);
                callback(result);
            }
        }
        let callback = clear_trampoline::<P>;
        unsafe {
            ffi::webkit_website_data_manager_clear(
                self.to_glib_none().0,
                types.into_glib(),
                timespan.into_glib(),
                cancellable.map(|p| p.as_ref()).to_glib_none().0,
                Some(callback),
                Box_::into_raw(user_data) as *mut _,
            );
        }
    }

    #[doc(alias = "webkit_website_data_manager_remove")]
    pub fn remove<P: FnOnce(Result<(), glib::Error>) + Send + 'static>(
        &self,
        types: WebsiteDataTypes,
        website_data: &[&WebsiteData],
        cancellable: Option<&impl IsA<gio::Cancellable>>,
        callback: P,
    ) {
        let user_data: Box_<P> = Box_::new(callback);
        unsafe extern "C" fn remove_trampoline<
            P: FnOnce(Result<(), glib::Error>) + Send + 'static,
        >(
            _source_object: *mut glib::gobject_ffi::GObject,
            res: *mut gio::ffi::GAsyncResult,
            user_data: glib::ffi::gpointer,
        ) {
            unsafe {
                let mut error = ptr::null_mut();
                let _ = ffi::webkit_website_data_manager_remove_finish(
                    _source_object as *mut _,
                    res,
                    &mut error,
                );
                let result = if error.is_null() {
                    Ok(())
                } else {
                    Err(from_glib_full(error))
                };
                let callback: Box_<P> = Box_::from_raw(user_data as *mut _);
                callback(result);
            }
        }
        let callback = remove_trampoline::<P>;
        unsafe {
            ffi::webkit_website_data_manager_remove(
                self.to_glib_none().0,
                types.into_glib(),
                website_data.to_glib_none().0,
                cancellable.map(|p| p.as_ref()).to_glib_none().0,
                Some(callback),
                Box_::into_raw(user_data) as *mut _,
            );
        }
    }
}
