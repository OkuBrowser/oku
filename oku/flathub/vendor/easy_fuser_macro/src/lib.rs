extern crate proc_macro;

mod fuse_handlers_signatures;

use either::Either;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, ExprBlock, Ident, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Async, Comma, FatArrow, Semi},
};

use fuse_handlers_signatures::*;

struct FnSigInput {
    attrs: Vec<Attribute>,
    asyncness: Option<Async>,
    name: Ident,
    tail: Either<Semi, ExprBlock>,
}

impl Parse for FnSigInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = Vec::new();
        while input.peek(Token![#]) {
            // Stop if it's an inner attribute #![...]
            if input.peek2(Token![!]) {
                break;
            }
            attrs.extend(input.call(Attribute::parse_outer)?);
        }

        let asyncness: Option<Async> = input.parse()?;
        let name: Ident = input.parse()?;
        let _arrow: FatArrow = input.parse()?;

        let tail = if input.peek(Token![;]) {
            Either::Left(input.parse::<Semi>()?)
        } else {
            Either::Right(input.parse::<ExprBlock>()?)
        };

        Ok(FnSigInput {
            attrs,
            asyncness,
            name,
            tail,
        })
    }
}

/// Usage:
/// 1. ```rust
///     fuse_handler_fnsig!{
///         /// doc...
///         init => { _default_implementation_ }
///     }
/// ```
/// 2. ```rust
///     fuse_handler_fnsig!{
///         /// doc...
///         access => ;
///     }
/// ```
#[proc_macro]
pub fn fuse_handler_fnsig(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as FnSigInput);
    let attrs = input.attrs;
    let asyncness = input.asyncness;
    let tail_tokens: proc_macro2::TokenStream = match input.tail {
        Either::Left(semi) => quote!( #semi ),
        Either::Right(block) => {
            quote!( #block )
        }
    };

    let trait_fn = get_fuse_handler_trait_fn(&input.name.to_string());
    let fn_sig = trait_fn.sig;

    quote!(
        #(#attrs)*
        #asyncness #fn_sig #tail_tokens
    )
    .into()
}

struct DelegateFsInput {
    target: Ident,
    _comma: Comma,
    methods: Punctuated<Ident, Comma>,
}
impl Parse for DelegateFsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let target = input.parse::<Ident>()?;
        let _comma = input.parse::<Comma>()?;
        let content;
        syn::bracketed!(content in input);
        let methods = Punctuated::<Ident, Comma>::parse_terminated(&content)?;
        Ok(Self {
            target,
            _comma,
            methods,
        })
    }
}

/// Synchronously delegates FUSE handler trait methods to a target field.
///
/// This macro is used in **serial** or **parallel** concurrency modes. It generates synchronous method implementations
/// that forward the calls directly to the specified field.
///
/// # Example
///
/// ```rust
/// struct MyFs {
///     mirror_fs: MirrorFs,
/// }
///
/// impl FuseHandler for MyFs {
///     type TId = PathBuf;
///
///     delegate_fs! { mirror_fs, [ read, write, getattr ] }
/// }
/// ```
#[proc_macro]
pub fn delegate_fs(input: TokenStream) -> TokenStream {
    delegate_fs_impl(false, input)
}

/// Asynchronously delegates FUSE handler trait methods to an **asynchronous** target field.
///
/// Use this macro in **async** mode when the delegate target itself has `async` methods (returning futures).
///
/// # Difference from `delegate_fs_sync_to_async`
/// - `delegate_fs_async` expects the target's methods to be async, wrapping the call in a pinned async block and `.await`ing the result.
/// - `delegate_fs_sync_to_async` is for synchronous targets, wrapping the blocking call in a pinned async block *without* `.await`ing.
///
/// # Manual Desugaring / `#[async_trait]`
/// Because outer attribute macros like `#[async_trait]` expand before inner delegation macros, `#[async_trait]` cannot see/rewrite the signatures inside our macro.
/// Consequently, this macro performs **manual signature desugaring**:
/// - Rewrites parameter references with explicit lifetimes (`'life0`, `'life1`, etc.).
/// - Rewrites the return type to `Pin<Box<dyn Future<Output = ...> + Send + 'async_trait>>`.
/// - Constrains parameter/self lifetimes to `'async_trait' in the generated `where` clause.
///
/// # Example
///
/// ```rust
/// #[async_trait]
/// impl FuseHandler for MyAsyncFs {
///     type TId = PathBuf;
///
///     delegate_fs_async! { async_mirror_fs, [ read, write ] }
/// }
/// ```
#[proc_macro]
pub fn delegate_fs_async(input: TokenStream) -> TokenStream {
    delegate_fs_impl_async_desugared(true, input)
}

/// Asynchronously delegates FUSE handler trait methods to a **synchronous** target field.
///
/// Use this macro in **async** mode when the delegate target has standard synchronous/blocking methods.
///
/// # Difference from `delegate_fs_async`
/// - `delegate_fs_sync_to_async` wraps the synchronous call in a pinned async block *without* calling `.await` (since the target method runs synchronously).
/// - `delegate_fs_async` expects the target's methods to be async, wrapping the call in a pinned async block and `.await`ing the result.
///
/// # Manual Desugaring / `#[async_trait]`
/// Because outer attribute macros like `#[async_trait]` expand before inner delegation macros, `#[async_trait]` cannot see/rewrite the signatures inside our macro.
/// Consequently, this macro performs **manual signature desugaring**:
/// - Rewrites parameter references with explicit lifetimes (`'life0`, `'life1`, etc.).
/// - Rewrites the return type to `Pin<Box<dyn Future<Output = ...> + Send + 'async_trait>>`.
/// - Constrains parameter/self lifetimes to `'async_trait` in the generated `where` clause.
///
/// # Example
///
/// ```rust
/// #[async_trait]
/// impl FuseHandler for MyAsyncFs {
///     type TId = PathBuf;
///
///     delegate_fs_sync_to_async! { sync_mirror_fs, [ getattr, readlink ] }
/// }
/// ```
#[proc_macro]
pub fn delegate_fs_sync_to_async(input: TokenStream) -> TokenStream {
    delegate_fs_impl_async_desugared(false, input)
}

fn delegate_fs_impl(is_async: bool, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as DelegateFsInput);

    let target = args.target;

    let mut expanded = quote! {};

    for method in &args.methods {
        let fn_impl = get_fuse_handler_trait_fn(&method.to_string());
        let method_expr = make_method_call_expr(&fn_impl);
        let fn_sig = fn_impl.sig;

        if !is_async {
            expanded.extend(quote! {
                #fn_sig {
                    self.#target.#method_expr
                }
            });
        } else {
            expanded.extend(quote! {
                async #fn_sig {
                    self.#target.#method_expr.await
                }
            });
        }
    }

    expanded.into()
}

fn delegate_fs_impl_async_desugared(is_target_async: bool, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as DelegateFsInput);
    let target = args.target;
    let mut expanded = quote! {};

    for method in &args.methods {
        let fn_impl = get_fuse_handler_trait_fn(&method.to_string());
        let method_expr = make_method_call_expr(&fn_impl);
        let original_sig = fn_impl.sig;

        let (desugared_sig, _) = desugar_signature(&original_sig);

        if is_target_async {
            expanded.extend(quote! {
                #desugared_sig {
                    ::std::boxed::Box::pin(async move {
                        self.#target.#method_expr.await
                    })
                }
            });
        } else {
            expanded.extend(quote! {
                #desugared_sig {
                    ::std::boxed::Box::pin(async move {
                        self.#target.#method_expr
                    })
                }
            });
        }
    }

    expanded.into()
}

fn desugar_signature(sig: &syn::Signature) -> (syn::Signature, Vec<syn::Lifetime>) {
    let mut new_sig = sig.clone();
    let mut lifetimes = Vec::new();
    let mut life_idx = 0;

    let mut next_life = || {
        let name = format!("'life{}", life_idx);
        life_idx += 1;
        let lt = syn::Lifetime::new(&name, proc_macro2::Span::call_site());
        lifetimes.push(lt.clone());
        lt
    };

    for input in &mut new_sig.inputs {
        match input {
            syn::FnArg::Receiver(receiver) => {
                if let Some((_, ref mut lifetime)) = receiver.reference {
                    *lifetime = Some(next_life());
                }
            }
            syn::FnArg::Typed(pat_type) => {
                rewrite_type(&mut pat_type.ty, &mut next_life);
            }
        }
    }

    let async_trait_lifetime = syn::Lifetime::new("'async_trait", proc_macro2::Span::call_site());

    for lt in &lifetimes {
        let gp: syn::GenericParam = syn::parse_quote!(#lt);
        new_sig.generics.params.push(gp);
    }
    let gp: syn::GenericParam = syn::parse_quote!(#async_trait_lifetime);
    new_sig.generics.params.push(gp);

    let mut where_clause = new_sig.generics.where_clause.take().unwrap_or_else(|| syn::WhereClause {
        where_token: syn::token::Where::default(),
        predicates: syn::punctuated::Punctuated::new(),
    });
    for lt in &lifetimes {
        where_clause.predicates.push(syn::parse_quote!(#lt: #async_trait_lifetime));
    }
    where_clause.predicates.push(syn::parse_quote!(Self: #async_trait_lifetime));
    new_sig.generics.where_clause = Some(where_clause);

    let original_output = match &new_sig.output {
        syn::ReturnType::Default => quote!(()),
        syn::ReturnType::Type(_, ty) => quote!(#ty),
    };
    new_sig.output = syn::parse_quote! {
        -> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #original_output> + ::std::marker::Send + 'async_trait>>
    };

    (new_sig, lifetimes)
}

fn rewrite_type(ty: &mut syn::Type, next_life: &mut dyn FnMut() -> syn::Lifetime) {
    match ty {
        syn::Type::Reference(type_ref) => {
            type_ref.lifetime = Some(next_life());
            rewrite_type(&mut type_ref.elem, next_life);
        }
        syn::Type::Path(type_path) => {
            for segment in &mut type_path.path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &mut segment.arguments {
                    for arg in &mut args.args {
                        match arg {
                            syn::GenericArgument::Lifetime(lt) => {
                                *lt = next_life();
                            }
                            syn::GenericArgument::Type(inner_ty) => {
                                rewrite_type(inner_ty, next_life);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
