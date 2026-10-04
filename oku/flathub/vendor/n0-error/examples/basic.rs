use n0_error::{Result, StackResultExt, StdResultExt, e, stack_error};

/// The `stack_error` macro controls how to turn our enum into a `StackError`.
///
/// * `add_meta` adds a field to all variants to track the call-site error location
/// * `derive` adds `#[derive(StackError)]`
/// * `from_sources` creates `From` impls for the error sources
#[stack_error(derive, add_meta, from_sources)]
enum MyError {
    /// We can define the error message with the `error` attribute
    /// It should not include the error source, those are printed in addition depending on the output format.
    #[error("invalid input")]
    InvalidInput { source: InvalidInput },
    /// Or we can define a variant as `transparent`, which forwards the Display impl to the error source
    #[error(transparent)]
    Io {
        /// For sources that do not implement `StackError`, we have to mark the source as `std_err`.
        #[error(std_err)]
        source: std::io::Error,
    },
}

/// We can use the [`stack_error`] macro on structs as well.
#[stack_error(derive, add_meta)]
#[error("wanted {expected} but got {actual}")]
struct InvalidInput {
    expected: u32,
    actual: u32,
}

fn validate_input(number: u32) -> Result<(), InvalidInput> {
    if number != 23 {
        // The `e` macro constructs a `StackError` while automatically adding the `meta` field.
        Err(e!(InvalidInput {
            actual: number,
            expected: 23
        }))
    } else {
        Ok(())
    }
}

fn fail_io() -> std::io::Result<()> {
    Err(std::io::Error::other("io failed"))
}

/// Some function that returns [`MyError`].
fn process(number: u32) -> Result<(), MyError> {
    // We have a `From` impl for `InvalidInput` on our error.
    validate_input(number)?;
    // We have a `From` impl for `std::io::Error` on our error.
    fail_io()?;
    // Without the From impl, we'd need to forward the error manually.
    // The `e` macro can assist here, so that we don't have to declare the `meta` field manually.
    fail_io().map_err(|source| e!(MyError::Io, source))?;
    Ok(())
}

// A main function that returns AnyError (via the crate's Result alias)
fn run(number: u32) -> Result<()> {
    // We can add context to errors via the result extensions.
    // The `context` function adds context to any `StackError`.
    process(number).context("failed to process input")?;
    // To add context to std errors, we have to use `std_context` from `StdResultExt`.
    fail_io().std_context("failed at fail_io")?;
    Ok(())
}

fn main() -> Result<()> {
    if let Err(err) = run(13) {
        println!("{err}");
        // failed to process input

        println!("{err:#}");
        // failed to process input: invalid input: wanted 23 but got 13

        println!("{err:?}");
        // failed to process input
        // Caused by:
        //     invalid input
        //     wanted 23 but got 13

        // and with RUST_BACKTRACE=1 or RUST_ERROR_LOCATION=1
        // failed to process input (examples/basic.rs:61:17)
        // Caused by:
        //     invalid input (examples/basic.rs:36:5)
        //     wanted 23 but got 13 (examples/basic.rs:48:13)

        println!("{err:#?}");
        // Stack(WithSource {
        //     message: "failed to process input",
        //     source: Stack(InvalidInput {
        //         source: BadNumber {
        //             expected: 23,
        //             actual: 13,
        //             meta: Meta(examples/basic.rs:48:13),
        //         },
        //         meta: Meta(examples/basic.rs:36:5),
        //     }),
        //     meta: Meta(examples/basic.rs:61:17),
        // })
    }
    Ok(())
}

/// You can also use the macros with tuple structs or enums.
/// In this case the meta field will be added as the last field.
#[stack_error(derive, add_meta)]
#[error("tuple fail ({_0})")]
struct TupleStruct(u32);

#[stack_error(derive, add_meta)]
#[allow(unused)]
enum TupleEnum {
    #[error("io failed")]
    Io(#[error(source, std_err)] std::io::Error),
}
