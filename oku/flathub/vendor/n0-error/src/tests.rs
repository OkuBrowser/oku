use std::io;

use self::util::wait_sequential;
use crate::{
    AnyError, Result, StackError, StackErrorExt, StackResultExt, StdResultExt, anyerr, e,
    ensure_any, meta, set_backtrace_enabled, stack_error,
};
mod util;

#[test]
fn infallible_from_source() {
    #[stack_error(derive, add_meta, from_sources)]
    enum Error {
        #[error(transparent)]
        Infallible {
            #[error(std_err)]
            source: std::convert::Infallible,
        },
    }

    #[stack_error(derive, add_meta)]
    #[error(transparent)]
    struct StructFrom {
        #[error(std_err, from)]
        source: std::convert::Infallible,
    }

    #[stack_error(derive, add_meta)]
    #[error(transparent)]
    struct StructNew {
        #[error(std_err)]
        source: std::convert::Infallible,
    }

    fn assert_from<T: From<std::convert::Infallible>>() {}
    assert_from::<Error>();
    assert_from::<StructFrom>();
    let _ = StructNew::new;
}

#[test]
#[cfg(feature = "anyhow")]
fn test_anyhow_compat() -> Result {
    fn ok() -> anyhow::Result<()> {
        Ok(())
    }
    ok().map_err(AnyError::from_anyhow)
}

#[stack_error(add_meta)]
#[derive(StackError)]
enum MyError {
    #[error("A failure")]
    A {},
}
#[test]
fn test_whatever() {
    let _guard = wait_sequential();

    fn fail() -> Result {
        n0_error::bail_any!("sad face");
    }

    fn fail_my_error() -> Result<(), MyError> {
        Err(MyError::A { meta: meta() })
    }

    fn fail_whatever() -> Result {
        n0_error::try_or_any!(fail(), "sad");
        Ok(())
    }

    fn fail_whatever_my_error() -> Result {
        n0_error::try_or_any!(fail_my_error(), "sad");
        Ok(())
    }

    n0_error::set_backtrace_enabled(false);
    assert!(fail().is_err());
    assert_eq!(format!("{:?}", fail().unwrap_err()), "sad face");
    assert_eq!(format!("{}", fail().unwrap_err()), "sad face");
    assert!(fail_my_error().is_err());
    assert_eq!(format!("{}", fail_my_error().unwrap_err()), "A failure");
    assert_eq!(format!("{:#}", fail_my_error().unwrap_err()), "A failure");
    assert_eq!(format!("{:?}", fail_my_error().unwrap_err()), "A failure");
    // assert_eq!(
    //     format!("{:#?}", fail_my_error().unwrap_err()),
    //     "A {\n    location: None,\n}"
    // );
    assert!(fail_whatever().is_err());
    assert!(fail_whatever_my_error().is_err());

    assert_eq!(
        format!("{:?}", fail_whatever().unwrap_err()),
        "sad\nCaused by:\n    sad face"
    );

    assert_eq!(
        format!("{:?}", fail_whatever()),
        "Err(sad\nCaused by:\n    sad face)"
    );

    n0_error::set_backtrace_enabled(true);

    assert!(fail_my_error().is_err());
    assert_eq!(format!("{}", fail_my_error().unwrap_err()), "A failure");
    assert_eq!(format!("{:#}", fail_my_error().unwrap_err()), "A failure");
    assert_eq!(
        format!("{:?}", fail_my_error().unwrap_err()),
        format!("A failure (src/tests.rs:65:32)")
    );
    //     let expected = r#"A {
    //     location: Some(
    //         Location {
    //             file: "src/tests.rs",
    //             line: 33,
    //             column: 13,
    //         },
    //     ),
    // }"#;
    //     assert_eq!(format!("{:#?}", fail_my_error().unwrap_err()), expected);
}

#[test]
fn test_context_none() {
    fn fail() -> Result {
        None.std_context("sad")
    }

    assert!(fail().is_err());
}

#[test]
fn test_format_err() {
    fn fail() -> Result {
        Err(anyerr!("sad: {}", 12))
    }

    assert!(fail().is_err());
}

#[test]
fn test_io_err() {
    fn fail_io() -> std::io::Result<()> {
        Err(std::io::Error::other("sad IO"))
    }

    fn fail_custom() -> Result<(), MyError> {
        Ok(())
    }

    fn fail_outer() -> Result {
        fail_io().anyerr()?;
        fail_custom()?;
        Ok(())
    }
    let err = fail_outer().unwrap_err();
    assert_eq!(err.to_string(), "sad IO");
}

#[test]
fn test_message() {
    fn fail_box() -> Result<(), impl std::error::Error + Send + Sync + 'static> {
        Err(Box::new(std::io::Error::other("foo")))
    }

    let my_res = fail_box().std_context("failed");

    let err = my_res.unwrap_err();
    assert_eq!(format!("{err:#}"), "failed: foo");
    let stack = err.stack();
    assert_eq!(stack.count(), 2);
}

#[test]
fn test_option() {
    fn fail_opt() -> Option<()> {
        None
    }

    let my_res = fail_opt().context("failed");

    let err = my_res.unwrap_err();
    assert_eq!(format!("{err:#}"), "failed: Expected some, found none");
    let stack = err.stack();
    assert_eq!(stack.count(), 2);
}

#[test]
fn test_sources() {
    let _guard = wait_sequential();
    n0_error::set_backtrace_enabled(false);
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let file_name = "foo.txt";
    let res: Result<(), _> = Err(err).with_std_context(|_| format!("failed to read {file_name}"));
    let res: Result<(), AnyError> = res.context("read error");

    let err = res.err().unwrap();

    let fmt = format!("{err}");
    println!("short:\n{fmt}\n");
    assert_eq!(&fmt, "read error");

    let fmt = format!("{err:#}");
    println!("alternate:\n{fmt}\n");
    assert_eq!(&fmt, "read error: failed to read foo.txt: file not found");

    let fmt = format!("{err:?}");
    println!("debug :\n{fmt}\n");
    assert_eq!(
        &fmt,
        r#"read error
Caused by:
    failed to read foo.txt
    file not found"#
    );

    let fmt = format!("{err:#?}");
    println!("debug alternate:\n{fmt}\n");

    n0_error::set_backtrace_enabled(true);

    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let file_name = "foo.txt";
    let res: Result<(), _> = Err(err).with_std_context(|_| format!("failed to read {file_name}"));
    let res: Result<(), _> = res.context("read error");

    let err = res.err().unwrap();

    let fmt = format!("{err}");
    println!("short:\n{fmt}\n");
    assert_eq!(&fmt, "read error");

    let fmt = format!("{err:#}");
    println!("alternate:\n{fmt}\n");
    assert_eq!(&fmt, "read error: failed to read foo.txt: file not found");

    let fmt = format!("{err:?}");
    println!("debug :\n{fmt}\n");
    assert_eq!(
        &fmt,
        r#"read error (src/tests.rs:226:34)
Caused by:
    failed to read foo.txt (src/tests.rs:225:39)
    file not found (src/tests.rs:225:39)"#
    );
    let fmt = format!("{err:#?}");
    println!("debug alternate:\n{fmt}\n");
}

#[test]
fn test_ensure() {
    let _guard = wait_sequential();
    fn foo() -> Result {
        ensure_any!(false, "sad face");
        Ok(())
    }
    let err = foo().unwrap_err();
    assert_eq!(format!("{err}"), "sad face")
}

#[derive(StackError)]
struct SomeError;

#[derive(StackError)]
#[error("fail ({code})")]
struct SomeErrorFields {
    code: u32,
}

#[test]
fn test_structs() {
    let _guard = wait_sequential();

    n0_error::set_backtrace_enabled(false);
    fn fail_some_error() -> Result<(), SomeError> {
        Err(SomeError)
    }

    fn fail_some_error_fields() -> Result<(), SomeErrorFields> {
        Err(SomeErrorFields { code: 22 })
    }

    let res = fail_some_error();
    let err = res.unwrap_err();
    assert_eq!(format!("{err}"), "SomeError");
    let err2 = err.context("bad");
    assert_eq!(format!("{err2:#}"), "bad: SomeError");

    let res = fail_some_error_fields();
    let err = res.unwrap_err();
    assert_eq!(format!("{err}"), "fail (22)");
    let err2 = err.context("bad");
    assert_eq!(format!("{err2:#}"), "bad: fail (22)");
}

#[stack_error(add_meta)]
#[derive(StackError)]
struct SomeErrorLoc;

#[stack_error(add_meta)]
#[derive(StackError)]
#[error("fail ({code})")]
struct SomeErrorLocFields {
    code: u32,
}

#[test]
fn test_structs_location() {
    let _guard = wait_sequential();
    n0_error::set_backtrace_enabled(true);
    fn fail_some_error() -> Result<(), SomeErrorLoc> {
        Err(SomeErrorLoc::new())
    }

    fn fail_some_error_fields() -> Result<(), SomeErrorLocFields> {
        Err(SomeErrorLocFields::new(22))
    }

    let res = fail_some_error();
    let err = res.unwrap_err();
    assert_eq!(format!("{err}"), "SomeErrorLoc");
    assert_eq!(format!("{err:?}"), "SomeErrorLoc (src/tests.rs:313:13)");
    let err2 = err.context("bad");
    assert_eq!(format!("{err2:#}"), "bad: SomeErrorLoc");
    let res = fail_some_error_fields();
    let err = res.unwrap_err();
    assert_eq!(format!("{err}"), "fail (22)");
    let err2 = err.context("bad");
    assert_eq!(format!("{err2:#}"), "bad: fail (22)");
    println!("{err2:?}");
    assert_eq!(
        format!("{err2:?}"),
        r#"bad (src/tests.rs:329:20)
Caused by:
    fail (22) (src/tests.rs:317:13)"#
    );
}

#[test]
fn test_context() {
    #[stack_error(add_meta)]
    #[derive(StackError)]
    enum AppError {
        My { source: MyError },
        Baz { source: MyError, count: usize },
    }
    // impl AppError {
    //     fn My() -> fn(MyError) -> Self {
    //         Self::my
    //     }

    //     fn Baz(count: usize) -> impl Fn(MyError) -> Self {
    //         move |source| Self::baz(source, count)
    //     }
    // }
    fn fail_a() -> Result<(), MyError> {
        Err(MyError::A { meta: meta() })
    }
    println!("{:?}", fail_a().std_context("foo").unwrap_err());
    println!("---");
    println!(
        "{:?}",
        fail_a().map_err(|s| e!(AppError::My, s)).unwrap_err()
    );
    println!("---");
    println!(
        "{:?}",
        fail_a()
            .map_err(|source| e!(AppError::Baz { source, count: 32 }))
            .unwrap_err()
    );
    println!("---");
    // println!("{:?}", fail_a().context2(AppError::baz).unwrap_err());
}

#[test]
fn test_any() {
    // fn foo() -> Result {
    //     Err(io::Error::other("foo"))?;
    //     Ok(())
    // }

    // let x = foo();
    let x = anyerr!("foo");
    println!("{x:?}");
    let x = anyerr!(e!(MyError::A));
    println!("{x:?}");
    let x = anyerr!(io::Error::other("foo"));
    println!("{x:?}");
}

// --- tuple support tests ---

#[stack_error(add_meta)]
#[derive(StackError)]
#[error("tuple fail ({_0})")]
struct TupleStruct(u32);

#[test]
fn test_tuple_struct_basic() {
    let _guard = wait_sequential();
    n0_error::set_backtrace_enabled(false);
    let err = TupleStruct::new(7);
    assert_eq!(format!("{err}"), "tuple fail (7)");
}

#[stack_error(add_meta)]
#[derive(StackError)]
#[error(from_sources)]
enum TupleEnum {
    #[error("io failed")]
    Io(#[error(source, std_err)] io::Error),
    #[error(transparent)]
    Transparent(MyError),
}

#[test]
fn test_tuple_enum_source_and_meta() {
    let _guard = wait_sequential();
    n0_error::set_backtrace_enabled(false);
    let e = TupleEnum::Io(io::Error::other("oops"), meta());
    // Display uses variant text
    assert_eq!(format!("{e}"), "io failed");
    // Std source is the inner io::Error
    let src = std::error::Error::source(&e).unwrap();
    assert_eq!(src.to_string(), "oops");

    let err = e!(MyError::A);
    let err = TupleEnum::from(err);
    assert_eq!(format!("{err}"), "A failure");
    assert_eq!(format!("{err:?}"), "A failure");
    n0_error::set_backtrace_enabled(true);
    let err = e!(MyError::A);
    let err = TupleEnum::from(err);
    assert_eq!(
        format!("{err:?}"),
        "TupleEnum::Transparent (src/tests.rs:435:15)\nCaused by:\n    A failure (src/tests.rs:434:15)"
    );
}

// TODO: turn into actual test
#[test]
pub fn test_skip_transparent_errors() {
    #[stack_error(add_meta)]
    #[derive(StackError)]
    #[error(from_sources)]
    enum ErrorA {
        #[error("failure at b")]
        ErrorB { source: ErrorB },
    }

    #[stack_error(add_meta)]
    #[derive(StackError)]
    #[error(std_sources)]
    enum ErrorB {
        #[error(transparent)]
        IoTransparent(io::Error),
        #[error("io error")]
        Io { source: io::Error },
    }

    fn err_a(transparent: bool) -> Result<(), ErrorA> {
        err_b(transparent)?;
        Ok(())
    }

    fn err_b(transparent: bool) -> Result<(), ErrorB> {
        if transparent {
            io().map_err(|err| ErrorB::IoTransparent(err, meta()))
        } else {
            io().map_err(|err| e!(ErrorB::Io, err))
        }
    }

    fn io() -> io::Result<()> {
        Err(io::Error::other("bad"))
    }

    let _guard = wait_sequential();
    set_backtrace_enabled(false);
    println!("#### no bt, transparent");
    println!("{:?}", err_a(true).unwrap_err());
    set_backtrace_enabled(true);
    println!("#### bt, transparent");
    println!("{:?}", err_a(true).unwrap_err());

    set_backtrace_enabled(false);
    println!("#### no bt, not transparent");
    println!("{:?}", err_a(false).unwrap_err());

    set_backtrace_enabled(true);
    println!("#### bt, transparent");
    println!("{:?}", err_a(true).unwrap_err());

    set_backtrace_enabled(true);
    println!("#### bt, transparent, display alt");
    println!("{:#}", err_a(true).unwrap_err());
    println!("#### bt, not transparent, display alt");
    println!("{:#}", err_a(false).unwrap_err());
    println!("#### bt, transparent, display");
    println!("{}", err_a(true).unwrap_err());
    println!("#### bt, not transparent, display");
    println!("{}", err_a(false).unwrap_err());

    set_backtrace_enabled(false);
    println!("#### no bt, transparent, display alt");
    println!("{:#}", err_a(true).unwrap_err());
    println!("#### no bt, not transparent, display alt");
    println!("{:#}", err_a(false).unwrap_err());
    println!("#### no bt, transparent, display");
    println!("{}", err_a(true).unwrap_err());
    println!("#### no bt, not transparent, display");
    println!("{}", err_a(false).unwrap_err());
}

#[test]
fn test_generics() {
    #[stack_error(add_meta)]
    #[derive(StackError)]
    #[error("failed at {}", list.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", "))]
    struct GenericError<E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static> {
        list: Vec<E>,
    }

    #[stack_error(add_meta)]
    #[derive(StackError)]
    enum GenericEnumError<E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static> {
        Foo,
        Bar {
            list: Vec<E>,
        },
        #[error("failed at {other}")]
        Baz {
            other: Box<E>,
        },
    }

    let err = GenericError::new(vec!["foo", "bar"]);
    assert_eq!(format!("{err}"), "failed at foo, bar");
    let err = e!(GenericEnumError::Baz {
        other: Box::new("foo")
    });
    assert_eq!(format!("{err}"), "failed at foo");
}

#[test]
fn downcast() {
    let err = e!(MyError::A).into_any();
    let err_ref: &MyError = err.downcast_ref().unwrap();
    assert!(matches!(err_ref, MyError::A { .. }));
    let err: MyError = err.downcast().unwrap();
    assert!(matches!(err, MyError::A { .. }));

    let err = anyerr!(io::Error::other("foo"));
    let err_ref: &io::Error = err.downcast_ref().unwrap();
    assert!(matches!(err_ref.kind(), io::ErrorKind::Other));
    let err: io::Error = err.downcast().unwrap();
    assert!(matches!(err.kind(), io::ErrorKind::Other));

    let err = e!(MyError::A).context("foo");
    let err_ref: &MyError = err.source().unwrap().downcast_ref().unwrap();
    assert!(matches!(err_ref, MyError::A { .. }));
}

#[test]
#[cfg(feature = "anyhow")]
fn test_anyhow_compat2() -> Result {
    fn ok() -> anyhow::Result<()> {
        Ok(())
    }
    ok().map_err(AnyError::from_anyhow)?;
    let _err = AnyError::from(anyhow::anyhow!("fail"));
    ok().context("ctx")?;
    ok()?;
    Ok(())
}

#[test]
fn test_struct_source() {
    fn inner() -> Result {
        #[stack_error(add_meta, derive)]
        struct NamedFields {
            source: AnyError,
            code: u32,
        }
        Err(NamedFields::new(anyerr!("foo"), 23))?;

        #[stack_error(add_meta, derive)]
        struct NamedFieldsStd {
            #[error(std_err)]
            source: io::Error,
            code: u32,
        }
        Err(NamedFieldsStd::new(io::Error::other("foo"), 23))?;

        #[stack_error(add_meta, derive)]
        struct TupleFields(#[error(source)] AnyError, u32);
        Err(TupleFields::new(anyerr!("foo"), 23))?;

        #[stack_error(add_meta, derive)]
        struct TupleFieldsStd(#[error(source, std_err)] io::Error, u32);
        Err(TupleFieldsStd::new(io::Error::other("foo"), 23))?;
        Ok(())
    }
    let _ = inner();
}
