//! # proc-macro-error
//!
//! This crate aims to make error reporting in proc-macros simple and easy to use.
//! Migrate from `panic!`-based errors for as little effort as possible!
//!
//! Also, there's ability to [append a dummy token stream](dummy/index.html) to your errors.
//!
//! ## Limitations
//!
//! - Warnings are emitted only on nightly, they are ignored on stable.
//! - "help" suggestions can't have their own span info on stable,
//!   (essentially inheriting the parent span).
//! - If a panic occurs somewhere in your macro no errors will be displayed. This is not a
//!   technical limitation but rather intentional design. `panic` is not for error reporting.
//! - Temporary incompatible with `proc_macro_hack`, unfortunately. No worries, some highly
//!   trained people are working on it!
//!
//! ## Guide
//!
//! ### Macros
//!
//! First of all - **all the emitting-related API must be used within a function
//! annotated with [`#[proc_macro_error]`](#proc_macro_error-attribute) attribute**. You'll just get a
//! panic otherwise, no errors will be shown.
//!
//! Most of the time you want to use the macros.
//!
//! - [`abort!`]:
//!
//!     Very much panic-like usage - abort execution and show the error. Expands to [`!`] (never type).
//!
//! - [`abort_call_site!`]:
//!
//!     Shortcut for `abort!(Span::call_site(), ...)`. Expands to [`!`] (never type).
//!
//! - [`emit_error!`]:
//!
//!     [`proc_macro::Diagnostic`]-like usage - emit the error but do not abort the macro.
//!     The compilation will fail nonetheless. Expands to [`()`] (unit type).
//!
//! - [`emit_call_site_error!`]:
//!
//!     Shortcut for `emit_error!(Span::call_site(), ...)`. Expands to [`()`] (unit type).
//!
//! - [`emit_warning!`]:
//!
//!     Like `emit_error!` but emit a warning instead of error. The compilation won't fail
//!     because of warnings.
//!     Expands to [`()`] (unit type).
//!
//!     **Beware**: warnings are nightly only, they are completely ignored on stable.
//!
//! - [`emit_call_site_warning!`]:
//!
//!     Shortcut for `emit_warning!(Span::call_site(), ...)`. Expands to [`()`] (unit type).
//!
//! - [`diagnostic`]:
//!
//!     Build instance of `Diagnostic` in format-like style.
//!
//! ### Syntax
//!
//! All the macros have pretty much the same syntax:
//!
//! 1.  ```ignore
//!     abort!(single_expr)
//!     ```
//!     Shortcut for `Diagnostic::from(expr).abort()`. **There's no way to attach notes
//!     in this form!**
//!
//! 2.  ```ignore
//!     abort!(span, message)
//!     ```
//!     The first argument is an expression the span info should be taken from. It can be
//!     either
//!
//!     * [`proc_macro::Span`]
//!     * [`proc_macro2::Span`]
//!     * Anything that implements [`quote::ToTokens`], in other words, almost every type
//!     * in `syn` and `proc_macro2`. **This form gives the best looking error messages and
//!       should be used whenever possible!**
//!
//!     The second argument is the error message, it must implement [`ToString`].
//!
//! 3.  ```ignore
//!     abort!(span, format_literal, format_args...)
//!     ```
//!
//!     This form is pretty much the same as 2, except `format!(format_literal, format_args...)`
//!     will be used to for the message instead of [`ToString`].
//!
//! That's it. `abort!`, `emit_warning`, `emit_error` share this exact syntax.
//!
//! `abort_call_site!`, `emit_call_site_warning`, `emit_call_site_error` lack 1 form
//! and do not take span in 2 and 3 forms. Those are essentially shortcuts for
//! `macro!(Span::call_site(), args...)`.
//!
//! `diagnostic!` requires `Level` instance between `span` and second argument (1 form is the same).
//!
//! > **Important!**
//! >
//! > If you have some type from `proc_macro` or `syn` to point to, do not call `.span()`
//! > on it but rather use it directly:
//! > ```no_run
//! > # use proc_macro_error::abort;
//! > # let input = proc_macro2::TokenStream::new();
//! > let ty: syn::Type = syn::parse2(input).unwrap();
//! > abort!(ty, "BOOM");
//! > //     ^^ <-- avoid .span()
//! > ```
//! >
//! > `.span()` calls work too, but you may experience regressions in message quality.
//!
//! #### Note attachments
//!
//! 3.  Every macro can have "note" attachments (only 2 and 3 form).
//!   ```ignore
//!   let opt_help = if have_some_info { Some("did you mean `this`?") } else { None };
//!
//!   abort!(
//!       span, message; // <--- attachments start with `;` (semicolon)
//!
//!       help = "format {} {}", "arg1", "arg2"; // <--- every attachment ends with `;`,
//!                                              //      maybe except the last one
//!
//!       note = "to_string"; // <--- one arg uses `.to_string()` instead of `format!()`
//!
//!       yay = "I see what {} did here", "you"; // <--- "help =" and "hint =" are mapped
//!                                              // to Diagnostic::help,
//!                                              // anything else is Diagnostic::note
//!
//!       wow = note_span => "custom span"; // <--- attachments can have their own span
//!                                         //      it takes effect only on nightly though
//!
//!       hint =? opt_help; // <-- "optional" attachment, get displayed only if `Some`
//!                         //     must be single `Option` expression
//!
//!       note =? note_span => opt_help // <-- optional attachments can have custom spans too
//!   );
//!   ```
//!
//! ### `#[proc_macro_error]` attribute
//!
//! **This attribute MUST be present on the top level of your macro.**
//!
//! This attribute performs the setup and cleanup necessary to make things work.
//!
//! #### Syntax
//!
//! `#[proc_macro_error]` or `#[proc_macro_error(settings...)]`, where `settings...`
//! is a comma-separated list of:
//!
//! - `proc_macro_hack`:
//!
//!     To correctly cooperate with `#[proc_macro_hack]` `#[proc_macro_error]`
//!     attribute must be placed *before* (above) it, like this:
//!
//!     ```ignore
//!     #[proc_macro_error]
//!     #[proc_macro_hack]
//!     #[proc_macro]
//!     fn my_macro(input: TokenStream) -> TokenStream {
//!         unimplemented!()
//!     }
//!     ```
//!
//!     If, for some reason, you can't place it like that you can use
//!     `#[proc_macro_error(proc_macro_hack)]` instead.
//!
//!     # Note
//!
//!     If `proc-macro-hack` was detected (by any means) `allow_not_macro`
//!     and `assert_unwind_safe` will be applied automatically.
//!
//! - `allow_not_macro`:
//!
//!     By default, the attribute checks that it's applied to a proc-macro.
//!     If none of `#[proc_macro]`, `#[proc_macro_derive]` nor `#[proc_macro_attribute]` are
//!     present it will panic. It's the intention - this crate is supposed to be used only with
//!     proc-macros.
//!
//!     This setting is made to bypass the check, useful in certain circumstances.
//!
//!     Please note: the function this attribute is applied to must return
//!     `proc_macro::TokenStream`.
//!
//!     This setting is implied if `proc-macro-hack` was detected.
//!
//! - `assert_unwind_safe`:
//!
//!     By default, your code must be [unwind safe]. If your code is not unwind safe,
//!     but you believe it's correct, you can use this setting to bypass the check.
//!     You would need this for code that uses `lazy_static` or `thread_local` with
//!     `Cell/RefCell` inside (and the like).
//!
//!     This setting is implied if `#[proc_macro_error]` is applied to a function
//!     marked as `#[proc_macro]`, `#[proc_macro_derive]` or `#[proc_macro_attribute]`.
//!
//!     This setting is also implied if `proc-macro-hack` was detected.
//!
//! ### Diagnostic type
//!
//! [`Diagnostic`] type is intentionally designed to be API compatible with [`proc_macro::Diagnostic`].
//! Not all API is implemented, only the part that can be reasonably implemented on stable.
//!
//!
//! [`abort!`]: macro.abort.html
//! [`emit_warning!`]: macro.emit_warning.html
//! [`emit_error!`]: macro.emit_error.html
//! [`abort_call_site!`]: macro.abort_call_site.html
//! [`emit_call_site_warning!`]: macro.emit_call_site_error.html
//! [`emit_call_site_error!`]: macro.emit_call_site_warning.html
//! [`diagnostic!`]: macro.diagnostic.html
//! [proc_macro_error]: ./../proc_macro_error_attr/attr.proc_macro_error.html
//! [`Diagnostic`]: struct.Diagnostic.html
//! [`proc_macro::Diagnostic`]: https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html
//! [unwind safe]: https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html#what-is-unwind-safety
//! [`!`]: https://doc.rust-lang.org/std/primitive.never.html
//! [`()`]: https://doc.rust-lang.org/std/primitive.unit.html

#![cfg_attr(not(use_fallback), feature(proc_macro_diagnostic))]
#![forbid(unsafe_code)]
#![allow(clippy::needless_doctest_main)]

// reexports for use in macros
#[doc(hidden)]
pub extern crate proc_macro;
#[doc(hidden)]
pub extern crate proc_macro2;

pub use crate::{
    diagnostic::{Diagnostic, Level},
    dummy::{append_dummy, set_dummy},
};
pub use proc_macro_error_attr::proc_macro_error;

use proc_macro2::TokenStream;
use quote::quote;

use std::cell::Cell;
use std::panic::{catch_unwind, resume_unwind, UnwindSafe};

pub mod dummy;

mod diagnostic;
mod macros;

#[cfg(use_fallback)]
#[path = "imp/fallback.rs"]
mod imp;

#[cfg(not(use_fallback))]
#[path = "imp/delegate.rs"]
mod imp;

/// This traits expands `Result<T, Into<Diagnostic>>` with some handy shortcuts.
pub trait ResultExt {
    type Ok;

    /// Behaves like `Result::unwrap`: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via `abort!`.
    fn unwrap_or_abort(self) -> Self::Ok;

    /// Behaves like `Result::expect`: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via `abort!`.
    /// If it aborts then resulting error message will be preceded with `message`.
    fn expect_or_abort(self, msg: &str) -> Self::Ok;
}

/// This traits expands `Option` with some handy shortcuts.
pub trait OptionExt {
    type Some;

    /// Behaves like `Option::expect`: if self is `Some` yield the contained value,
    /// otherwise abort macro execution via `abort_call_site!`.
    /// If it aborts the `message` will be used for [`compile_error!`][compl_err] invocation.
    ///
    /// [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
    fn expect_or_abort(self, msg: &str) -> Self::Some;
}

/// Abort macro execution and display all the emitted errors, if any.
///
/// Does nothing if no errors were emitted (warnings do not count).
pub fn abort_if_dirty() {
    imp::abort_if_dirty();
}

impl<T, E: Into<Diagnostic>> ResultExt for Result<T, E> {
    type Ok = T;

    fn unwrap_or_abort(self) -> T {
        match self {
            Ok(res) => res,
            Err(e) => e.into().abort(),
        }
    }

    fn expect_or_abort(self, message: &str) -> T {
        match self {
            Ok(res) => res,
            Err(e) => {
                let mut e = e.into();
                e.msg = format!("{}: {}", message, e.msg);
                e.abort()
            }
        }
    }
}

impl<T> OptionExt for Option<T> {
    type Some = T;

    fn expect_or_abort(self, message: &str) -> T {
        match self {
            Some(res) => res,
            None => abort_call_site!(message),
        }
    }
}

/// This is the entry point for a proc-macro.
///
/// **NOT PUBLIC API, SUBJECT TO CHANGE WITHOUT ANY NOTICE**
#[doc(hidden)]
pub fn entry_point<F>(f: F, proc_macro_hack: bool) -> proc_macro::TokenStream
where
    F: FnOnce() -> proc_macro::TokenStream + UnwindSafe,
{
    ENTERED_ENTRY_POINT.with(|flag| flag.set(true));
    let caught = catch_unwind(f);
    let dummy = dummy::cleanup();
    let err_storage = imp::cleanup();
    ENTERED_ENTRY_POINT.with(|flag| flag.set(false));

    let mut appendix = TokenStream::new();
    if proc_macro_hack {
        appendix.extend(quote! {
            #[allow(unused)]
            macro_rules! proc_macro_call {
                () => ( unimplemented!() )
            }
        });
    }

    match caught {
        Ok(ts) => {
            if err_storage.is_empty() {
                ts
            } else {
                quote!( #(#err_storage)* #dummy #appendix ).into()
            }
        }

        Err(boxed) => match boxed.downcast::<AbortNow>() {
            Ok(_) => quote!( #(#err_storage)* #dummy #appendix ).into(),
            Err(boxed) => resume_unwind(boxed),
        },
    }
}

fn abort_now() -> ! {
    check_correctness();
    panic!(AbortNow)
}

thread_local! {
    static ENTERED_ENTRY_POINT: Cell<bool> = Cell::new(false);
}

struct AbortNow;

fn check_correctness() {
    if !ENTERED_ENTRY_POINT.with(|flag| flag.get()) {
        panic!(
            "proc-macro-error API cannot be used outside of `entry_point` invocation, \
             perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
        );
    }
}

/// **ALL THE STUFF INSIDE IS NOT PUBLIC API!!!**
#[doc(hidden)]
pub mod __export {
    use proc_macro2::Span;
    use quote::ToTokens;

    // inspired by
    // https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md#simple-application

    pub trait DoubleSpanToTokens {
        #[allow(non_snake_case)]
        fn FIRST_ARG_MUST_EITHER_BE_SPAN_OR_IMPLEMENT_TO_TOKENS(&self) -> (Span, Span);
    }

    pub trait DoubleSpanSingleSpan2 {
        #[allow(non_snake_case)]
        fn FIRST_ARG_MUST_EITHER_BE_SPAN_OR_IMPLEMENT_TO_TOKENS(&self) -> (Span, Span);
    }

    pub trait DoubleSpanSingleSpan {
        #[allow(non_snake_case)]
        fn FIRST_ARG_MUST_EITHER_BE_SPAN_OR_IMPLEMENT_TO_TOKENS(&self) -> (Span, Span);
    }

    impl<T: ToTokens> DoubleSpanToTokens for &T {
        fn FIRST_ARG_MUST_EITHER_BE_SPAN_OR_IMPLEMENT_TO_TOKENS(&self) -> (Span, Span) {
            let mut ts = self.to_token_stream().into_iter();
            let start = ts
                .next()
                .map(|tt| tt.span())
                .unwrap_or_else(Span::call_site);
            let end = ts.last().map(|tt| tt.span()).unwrap_or(start);
            (start, end)
        }
    }

    impl DoubleSpanSingleSpan2 for Span {
        fn FIRST_ARG_MUST_EITHER_BE_SPAN_OR_IMPLEMENT_TO_TOKENS(&self) -> (Span, Span) {
            (*self, *self)
        }
    }

    impl DoubleSpanSingleSpan for proc_macro::Span {
        fn FIRST_ARG_MUST_EITHER_BE_SPAN_OR_IMPLEMENT_TO_TOKENS(&self) -> (Span, Span) {
            (self.clone().into(), self.clone().into())
        }
    }
}
