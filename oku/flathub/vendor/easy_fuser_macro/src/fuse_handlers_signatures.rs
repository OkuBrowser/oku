use syn::{Expr, ExprCall, ExprPath, TraitItemFn, parse_quote};

pub fn get_fuse_handler_trait_fn(func_name: &str) -> TraitItemFn {
    match func_name {
        "get_default_ttl" => parse_quote! {
            fn get_default_ttl(&self) -> Duration;
        },
        "init" => parse_quote! {
            fn init(&self, req: &RequestInfo, config: &mut KernelConfig) -> FuseResult<()>;
        },
        "destroy" => parse_quote! {
            fn destroy(&self);
        },
        "access" => parse_quote! {
            fn access(&self, req: &RequestInfo, file_id: Self::TId, mask: AccessFlags) -> FuseResult<()>;
        },
        "bmap" => parse_quote! {
            fn bmap(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                blocksize: u32,
                idx: u64,
            ) -> FuseResult<u64>;
        },
        "copy_file_range" => parse_quote! {
            fn copy_file_range(
                &self,
                req: &RequestInfo,
                file_in: Self::TId,
                file_handle_in: BorrowedFileHandle<'_>,
                offset_in: u64,
                file_out: Self::TId,
                file_handle_out: BorrowedFileHandle<'_>,
                offset_out: u64,
                len: u64,
                flags: CopyFileRangeFlags,
            ) -> FuseResult<u32>;
        },
        "create" => parse_quote! {
            fn create(
                &self,
                req: &RequestInfo,
                parent_id: Self::TId,
                name: &std::ffi::OsStr,
                mode: u32,
                umask: u32,
                flags: OpenFlags,
            ) -> FuseResult<(OwnedFileHandle, <Self::TId as FileIdType>::Metadata, FopenFlags)>;
        },
        "fallocate" => parse_quote! {
            fn fallocate(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                offset: i64,
                length: i64,
                mode: FallocateFlags,
            ) -> FuseResult<()>;
        },
        "flush" => parse_quote! {
            fn flush(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                lock_owner: u64,
            ) -> FuseResult<()>;
        },
        "forget" => parse_quote! {
            fn forget(&self, req: &RequestInfo, file_id: Self::TId, nlookup: u64);
        },
        "fsync" => parse_quote! {
            fn fsync(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                datasync: bool,
            ) -> FuseResult<()>;
        },
        "fsyncdir" => parse_quote! {
            fn fsyncdir(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                datasync: bool,
            ) -> FuseResult<()>;
        },
        "getattr" => parse_quote! {
            fn getattr(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: Option<BorrowedFileHandle<'_>>,
            ) -> FuseResult<FileAttribute>;
        },
        "getlk" => parse_quote! {
            fn getlk(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                lock_owner: u64,
                lock_info: LockInfo,
            ) -> FuseResult<LockInfo>;
        },
        "getxattr" => parse_quote! {
            fn getxattr(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                name: &std::ffi::OsStr,
                size: u32,
            ) -> FuseResult<Vec<u8>>;
        },
        "ioctl" => parse_quote! {
            fn ioctl(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                flags: IoctlFlags,
                cmd: u32,
                in_data: Vec<u8>,
                out_size: u32,
            ) -> FuseResult<(i32, Vec<u8>)>;
        },
        "link" => parse_quote! {
            fn link(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                newparent: Self::TId,
                newname: &std::ffi::OsStr,
            ) -> FuseResult<<Self::TId as FileIdType>::Metadata>;
        },
        "listxattr" => parse_quote! {
            fn listxattr(&self, req: &RequestInfo, file_id: Self::TId, size: u32) -> FuseResult<Vec<u8>>;
        },
        "lookup" => parse_quote! {
            fn lookup(
                &self,
                req: &RequestInfo,
                parent_id: Self::TId,
                name: &std::ffi::OsStr,
            ) -> FuseResult<<Self::TId as FileIdType>::Metadata>;
        },
        "lseek" => parse_quote! {
            fn lseek(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                seek: SeekFrom,
            ) -> FuseResult<i64>;
        },
        "mkdir" => parse_quote! {
            fn mkdir(
                &self,
                req: &RequestInfo,
                parent_id: Self::TId,
                name: &std::ffi::OsStr,
                mode: u32,
                umask: u32,
            ) -> FuseResult<<Self::TId as FileIdType>::Metadata>;
        },
        "mknod" => parse_quote! {
            fn mknod(
                &self,
                req: &RequestInfo,
                parent_id: Self::TId,
                name: &std::ffi::OsStr,
                mode: u32,
                umask: u32,
                rdev: DeviceType,
            ) -> FuseResult<<Self::TId as FileIdType>::Metadata>;
        },
        "open" => parse_quote! {
            fn open(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                flags: OpenFlags,
            ) -> FuseResult<(OwnedFileHandle, FopenFlags)>;
        },
        "opendir" => parse_quote! {
            fn opendir(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                flags: OpenFlags,
            ) -> FuseResult<(OwnedFileHandle, FopenFlags)>;
        },
        "read" => parse_quote! {
            fn read(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                seek: SeekFrom,
                size: u32,
                flags: OpenFlags,
                lock_owner: Option<u64>,
            ) -> FuseResult<Vec<u8>>;
        },
        "readdir" => parse_quote! {
            fn readdir(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
            ) -> FuseResult<Vec<(std::ffi::OsString, <Self::TId as FileIdType>::MinimalMetadata)>>;
        },
        "readdirplus" => parse_quote! {
            fn readdirplus(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle,
            ) -> FuseResult<Vec<(std::ffi::OsString, <Self::TId as FileIdType>::Metadata)>>;
        },
        "readlink" => parse_quote! {
            fn readlink(&self, req: &RequestInfo, file_id: Self::TId) -> FuseResult<Vec<u8>>;
        },
        "release" => parse_quote! {
            fn release(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: OwnedFileHandle,
                flags: OpenFlags,
                lock_owner: Option<u64>,
                flush: bool,
            ) -> FuseResult<()>;
        },
        "releasedir" => parse_quote! {
            fn releasedir(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: OwnedFileHandle,
                flags: OpenFlags,
            ) -> FuseResult<()>;
        },
        "removexattr" => parse_quote! {
            fn removexattr(&self, req: &RequestInfo, file_id: Self::TId, name: &std::ffi::OsStr) -> FuseResult<()>;
        },
        "rename" => parse_quote! {
            fn rename(
                &self,
                req: &RequestInfo,
                parent_id: Self::TId,
                name: &std::ffi::OsStr,
                newparent: Self::TId,
                newname: &std::ffi::OsStr,
                flags: RenameFlags,
            ) -> FuseResult<()>;
        },
        "rmdir" => parse_quote! {
            fn rmdir(&self, req: &RequestInfo, parent_id: Self::TId, name: &std::ffi::OsStr) -> FuseResult<()>;
        },
        "setattr" => parse_quote! {
            fn setattr(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                attrs: SetAttrRequest<'_>,
            ) -> FuseResult<FileAttribute>;
        },
        "setlk" => parse_quote! {
            fn setlk(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                lock_owner: u64,
                lock_info: LockInfo,
                sleep: bool,
            ) -> FuseResult<()>;
        },
        "setxattr" => parse_quote! {
            fn setxattr(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                name: &std::ffi::OsStr,
                value: Vec<u8>,
                flags: SetXAttrFlags,
                position: u32,
            ) -> FuseResult<()>;
        },
        "statfs" => parse_quote! {
            fn statfs(&self, req: &RequestInfo, file_id: Self::TId) -> FuseResult<StatFs>;
        },
        "symlink" => parse_quote! {
            fn symlink(
                &self,
                req: &RequestInfo,
                parent_id: Self::TId,
                link_name: &std::ffi::OsStr,
                target: &std::path::Path,
            ) -> FuseResult<<Self::TId as FileIdType>::Metadata>;
        },
        "write" => parse_quote! {
            fn write(
                &self,
                req: &RequestInfo,
                file_id: Self::TId,
                file_handle: BorrowedFileHandle<'_>,
                seek: SeekFrom,
                data: Vec<u8>,
                write_flags: WriteFlags,
                flags: OpenFlags,
                lock_owner: Option<u64>,
            ) -> FuseResult<u32>;
        },
        "unlink" => parse_quote! {
            fn unlink(&self, req: &RequestInfo, parent_id: Self::TId, name: &std::ffi::OsStr) -> FuseResult<()>;
        },
        _ => panic!("unknown function signature"),
    }
}

/// Given a trait method like:
///     fn access(&self, req: &RequestInfo, file_id: #TId, mask: AccessMask) -> FuseResult<()>
/// Returns an expression: `access(req, file_id, mask)`
pub fn make_method_call_expr(method: &TraitItemFn) -> Expr {
    let method_name = &method.sig.ident;

    let args: Vec<Expr> = method
        .sig
        .inputs
        .iter()
        .skip(1) // skip &self / self
        .map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => {
                let pat = &pat_type.pat;
                // We just use the pattern as-is (usually an ident)
                parse_quote!(#pat)
            }
            syn::FnArg::Receiver(_) => unreachable!("&self should have been skipped"),
        })
        .collect();

    

    Expr::Call(ExprCall {
        attrs: vec![],
        func: Box::new(Expr::Path(ExprPath {
            attrs: vec![],
            qself: None,
            path: syn::Path::from(method_name.clone()),
        })),
        paren_token: syn::token::Paren::default(),
        args: syn::punctuated::Punctuated::from_iter(args),
    })
}