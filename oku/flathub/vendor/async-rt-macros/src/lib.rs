//! Attribute macros for `async-rt`.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Error, Expr, ExprLit, ItemFn, Lit, Meta, Token, parse2, spanned::Spanned};

#[derive(Clone, Copy)]
enum AttributeKind {
    Main,
    Test,
}

impl AttributeKind {
    fn is_test(self) -> bool {
        matches!(self, Self::Test)
    }
}

#[derive(Clone, Copy, Default)]
enum Executor {
    #[default]
    Global,
    Tokio,
    Smol,
    Compio,
    ThreadPool,
    Lite,
}

impl Executor {
    fn parse(value: &str, span: Span) -> syn::Result<Self> {
        match value {
            "global" => Ok(Self::Global),
            "tokio" => Ok(Self::Tokio),
            "smol" => Ok(Self::Smol),
            "compio" => Ok(Self::Compio),
            "threadpool" | "thread_pool" => Ok(Self::ThreadPool),
            "lite" => Ok(Self::Lite),
            _ => Err(Error::new(
                span,
                "unknown executor. expected `global`, `tokio`, `smol`, `compio`, `threadpool`, or `lite`",
            )),
        }
    }

    fn drive(
        self,
        kind: AttributeKind,
        runtime_crate: &TokenStream2,
        body: &syn::Block,
    ) -> TokenStream2 {
        let builtin_executor = match self {
            Self::Tokio => quote!(#runtime_crate::global::BuiltinExecutor::Tokio),
            Self::Smol => quote!(#runtime_crate::global::BuiltinExecutor::Smol),
            Self::Compio => quote!(#runtime_crate::global::BuiltinExecutor::Compio),
            Self::ThreadPool => quote!(#runtime_crate::global::BuiltinExecutor::ThreadPool),
            Self::Lite => quote!(#runtime_crate::global::BuiltinExecutor::Lite),
            Self::Global => unreachable!("the global executor must be resolved before expansion"),
        };
        let create_executor = match self {
            Self::Tokio if kind.is_test() => quote! {
                #runtime_crate::rt::tokio::TokioRuntimeExecutor::with_single_thread()
                    .expect("async-rt failed to create the Tokio test runtime")
            },
            Self::Tokio => quote! {
                #runtime_crate::rt::tokio::TokioRuntimeExecutor::with_multi_thread()
                    .expect("async-rt failed to create the Tokio runtime")
            },
            Self::Smol => quote! {
                #runtime_crate::rt::smol::SmolRuntimeExecutor::new()
            },
            Self::Compio => quote! {
                #runtime_crate::rt::compio::CompioRuntimeExecutor::new()
                    .expect("async-rt failed to create the Compio runtime")
            },
            Self::ThreadPool => quote! {
                #runtime_crate::rt::threadpool::ThreadPoolExecutor
            },
            Self::Lite => quote! {
                #runtime_crate::rt::lite::LiteExecutor
            },
            Self::Global => unreachable!("the global executor must be resolved before expansion"),
        };

        quote! {
            let __async_rt_body = async move #body;
            #[allow(
                clippy::diverging_sub_expression,
                clippy::expect_used,
                clippy::needless_return,
                clippy::unwrap_in_result
            )]
            {
                let __async_rt_executor = #runtime_crate::global::ConfiguredExecutor::with_task_executor(
                    #create_executor,
                    #builtin_executor,
                );
                return #runtime_crate::ExecutorBlockOn::block_on(
                    &__async_rt_executor,
                    __async_rt_body,
                );
            }
        }
    }
}

#[derive(Default)]
struct Arguments {
    executor: Executor,
    executor_set: bool,
    driver: Option<Expr>,
}

impl Arguments {
    fn parse(tokens: TokenStream2) -> syn::Result<Self> {
        let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
        let arguments = parser.parse2(tokens)?;
        let mut result = Self::default();

        for argument in arguments {
            let Meta::NameValue(argument) = argument else {
                return Err(Error::new(
                    argument.span(),
                    "expected `executor = \"...\"` or `driver = ...`",
                ));
            };

            if argument.path.is_ident("driver") {
                if result.driver.is_some() {
                    return Err(Error::new(
                        argument.path.span(),
                        "duplicate `driver` option",
                    ));
                }
                if result.executor_set {
                    return Err(Error::new(
                        argument.path.span(),
                        "`driver` and `executor` cannot be used together",
                    ));
                }

                result.driver = Some(argument.value);
                continue;
            }

            if !argument.path.is_ident("executor") {
                return Err(Error::new(
                    argument.path.span(),
                    "unknown option. expected `executor` or `driver`",
                ));
            }
            if result.executor_set {
                return Err(Error::new(
                    argument.path.span(),
                    "duplicate `executor` option",
                ));
            }
            if result.driver.is_some() {
                return Err(Error::new(
                    argument.path.span(),
                    "`driver` and `executor` cannot be used together",
                ));
            }

            let (value, span) = match argument.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) => (value.value(), value.span()),
                Expr::Path(value) if value.path.get_ident().is_some() => {
                    let ident = value.path.get_ident().expect("checked above");
                    (ident.to_string(), ident.span())
                }
                value => {
                    return Err(Error::new(
                        value.span(),
                        "executor must be a string or identifier",
                    ));
                }
            };

            result.executor = Executor::parse(&value, span)?;
            result.executor_set = true;
        }

        Ok(result)
    }
}

macro_rules! entry_points {
    ($main:ident, $test:ident, $executor:ident) => {
        /// Runs an async function as a synchronous entry point.
        ///
        /// The `async-rt` crate selects the default executor when it re-exports
        /// this macro. A built-in executor can be selected explicitly with, for
        /// example, `#[async_rt::main(executor = "compio")]`.
        ///
        /// An explicit selection controls the runtime driving this function and the
        /// executor used by `async_rt::task`.
        ///
        /// A custom driver can be supplied with `driver = expression`. It drives the
        /// annotated future without changing the executor used by `async_rt::task`.
        #[proc_macro_attribute]
        pub fn $main(arguments: TokenStream, item: TokenStream) -> TokenStream {
            expand(
                arguments,
                item,
                AttributeKind::Main,
                Some(Executor::$executor),
            )
        }

        /// Runs an async function as a synchronous test.
        ///
        /// The `async-rt` crate selects the default executor when it re-exports
        /// this macro. A built-in executor can be selected explicitly with, for
        /// example, `#[async_rt::test(executor = "tokio")]`.
        ///
        /// An explicit selection controls the runtime driving this function and the
        /// executor used by `async_rt::task`.
        /// Tests using different executors in the same binary run one at a time.
        ///
        /// A custom driver can be supplied with `driver = expression`. It drives the
        /// annotated future without changing the executor used by `async_rt::task`.
        #[proc_macro_attribute]
        pub fn $test(arguments: TokenStream, item: TokenStream) -> TokenStream {
            expand(
                arguments,
                item,
                AttributeKind::Test,
                Some(Executor::$executor),
            )
        }
    };
}

entry_points!(main_tokio, test_tokio, Tokio);
entry_points!(main_smol, test_smol, Smol);
entry_points!(main_compio, test_compio, Compio);
entry_points!(main_threadpool, test_threadpool, ThreadPool);
entry_points!(main_lite, test_lite, Lite);

/// Runs an async function when no default runtime is enabled.
#[proc_macro_attribute]
pub fn main_fail(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand(arguments, item, AttributeKind::Main, None)
}

/// Runs an async test when no default runtime is enabled.
#[proc_macro_attribute]
pub fn test_fail(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand(arguments, item, AttributeKind::Test, None)
}

macro_rules! failure_entry_points {
    ($main:ident, $test:ident, $message:literal) => {
        /// Emits an error because no supported runtime is available.
        #[proc_macro_attribute]
        pub fn $main(_arguments: TokenStream, _item: TokenStream) -> TokenStream {
            Error::new(Span::call_site(), $message)
                .into_compile_error()
                .into()
        }

        /// Emits an error because no supported test runtime is available.
        #[proc_macro_attribute]
        pub fn $test(_arguments: TokenStream, _item: TokenStream) -> TokenStream {
            Error::new(Span::call_site(), $message)
                .into_compile_error()
                .into()
        }
    };
}

failure_entry_points!(
    main_wasm_fail,
    test_wasm_fail,
    "async-rt's `main` and `test` macros do not yet support wasm32"
);

fn expand(
    arguments: TokenStream,
    item: TokenStream,
    kind: AttributeKind,
    default_executor: Option<Executor>,
) -> TokenStream {
    expand_inner(arguments.into(), item.into(), kind, default_executor)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_inner(
    arguments: TokenStream2,
    item: TokenStream2,
    kind: AttributeKind,
    default_executor: Option<Executor>,
) -> syn::Result<TokenStream2> {
    let arguments = Arguments::parse(arguments)?;
    let mut function: ItemFn = parse2(item)?;

    if function.sig.asyncness.take().is_none() {
        return Err(Error::new(
            function.sig.fn_token.span,
            "the `async` keyword is required",
        ));
    }

    let runtime_crate = runtime_crate()?;
    let test_attribute = match kind {
        AttributeKind::Main => TokenStream2::new(),
        AttributeKind::Test => quote!(#[::core::prelude::v1::test]),
    };
    let attributes = function.attrs;
    let visibility = function.vis;
    let signature = function.sig;
    let body = function.block;
    let drive = match arguments.driver {
        Some(driver) => drive_custom(&runtime_crate, &body, driver),
        None => {
            let executor = match arguments.executor {
                Executor::Global => default_executor.ok_or_else(|| {
                    Error::new(
                        Span::call_site(),
                        "async-rt's `main` and `test` macros require a default runtime feature, an explicit executor, or a custom `driver`",
                    )
                })?,
                executor => executor,
            };
            executor.drive(kind, &runtime_crate, &body)
        }
    };

    Ok(quote! {
        #(#attributes)*
        #test_attribute
        #visibility #signature {
            #drive
        }
    })
}

fn drive_custom(runtime_crate: &TokenStream2, body: &syn::Block, driver: Expr) -> TokenStream2 {
    quote! {
        let __async_rt_body = async move #body;
        let __async_rt_executor = #runtime_crate::global::ConfiguredExecutor::new(#driver);
        return #runtime_crate::ExecutorBlockOn::block_on(
            &__async_rt_executor,
            __async_rt_body,
        );
    }
}

fn runtime_crate() -> syn::Result<TokenStream2> {
    match crate_name("async-rt") {
        Ok(FoundCrate::Itself) => Ok(quote!(::async_rt)),
        Ok(FoundCrate::Name(name)) => {
            let name = format_ident!("{}", name.replace('-', "_"));
            Ok(quote!(::#name))
        }
        Err(error) => Err(Error::new(
            Span::call_site(),
            format!("could not find the `async-rt` crate: {error}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Arguments, Executor};
    use quote::quote;
    use syn::Expr;

    #[test]
    fn parses_string_and_identifier_executors() {
        assert!(matches!(
            Arguments::parse(quote!(executor = "compio"))
                .unwrap()
                .executor,
            Executor::Compio
        ));
        assert!(matches!(
            Arguments::parse(quote!(executor = tokio)).unwrap().executor,
            Executor::Tokio
        ));
        assert!(matches!(
            Arguments::parse(quote!(executor = "lite"))
                .unwrap()
                .executor,
            Executor::Lite
        ));
    }

    #[test]
    fn rejects_unknown_options_and_executors() {
        assert!(Arguments::parse(quote!(flavor = "current_thread")).is_err());
        assert!(Arguments::parse(quote!(executor = "unknown")).is_err());
    }

    #[test]
    fn parses_custom_driver_expression() {
        let arguments = Arguments::parse(quote!(driver = custom::executor())).unwrap();
        assert!(matches!(arguments.driver, Some(Expr::Call(_))));
    }

    #[test]
    fn rejects_duplicate_or_conflicting_driver_options() {
        assert!(Arguments::parse(quote!(driver = first(), driver = second())).is_err());
        assert!(Arguments::parse(quote!(executor = "tokio", driver = custom())).is_err());
        assert!(Arguments::parse(quote!(driver = custom(), executor = "tokio")).is_err());
    }
}
