//! This is `#[proc_macro_error]` attribute to be used with
//! [`proc-macro-error`](https://docs.rs/proc-macro-error/). There you go.

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use std::iter::FromIterator;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Attribute, Token,
};
use syn_mid::{Block, ItemFn};

use self::Setting::*;

#[proc_macro_attribute]
pub fn proc_macro_error(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let mut settings = match syn::parse::<Settings>(attr) {
        Ok(settings) => settings,
        Err(err) => {
            let err = err.to_compile_error();
            return quote!(#input #err).into();
        }
    };

    let is_proc_macro = is_proc_macro(&input.attrs);
    if is_proc_macro {
        settings.set(AssertUnwindSafe);
    }

    if detect_proc_macro_hack(&input.attrs) {
        settings.set(ProcMacroHack);
    }

    if settings.is_set(ProcMacroHack) {
        settings.set(AllowNotMacro);
    }

    if !(settings.is_set(AllowNotMacro) || is_proc_macro) {
        return quote!(
            #input
            compile_error!(
                "#[proc_macro_error] attribute can be used only with a proc-macro\n\n  \
                = hint: if you are really sure that #[proc_macro_error] should be applied \
                to this exact function use #[proc_macro_error(allow_not_macro)]\n");
        )
        .into();
    }

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = input;

    let body = gen_body(*block, settings);

    quote!(
        #(#attrs)*
        #vis
        #sig
        { #body }
    )
    .into()
}

#[derive(PartialEq)]
enum Setting {
    AssertUnwindSafe,
    AllowNotMacro,
    ProcMacroHack,
}

impl Parse for Setting {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match &*ident.to_string() {
            "assert_unwind_safe" => Ok(AssertUnwindSafe),
            "allow_not_macro" => Ok(AllowNotMacro),
            "proc_macro_hack" => Ok(ProcMacroHack),
            _ => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown setting `{}`, expected one of \
                     `assert_unwind_safe`, `allow_not_macro`, `proc_macro_hack`",
                    ident
                ),
            )),
        }
    }
}

struct Settings(Vec<Setting>);
impl Parse for Settings {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let punct = Punctuated::<Setting, Token![,]>::parse_terminated(input)?;
        Ok(Settings(Vec::from_iter(punct)))
    }
}

impl Settings {
    fn is_set(&self, setting: Setting) -> bool {
        self.0.iter().any(|s| *s == setting)
    }

    fn set(&mut self, setting: Setting) {
        self.0.push(setting)
    }
}

#[cfg(not(always_assert_unwind))]
fn gen_body(block: Block, settings: Settings) -> proc_macro2::TokenStream {
    let is_proc_macro_hack = settings.is_set(ProcMacroHack);
    let closure = if settings.is_set(AssertUnwindSafe) {
        quote!(::std::panic::AssertUnwindSafe(|| #block ))
    } else {
        quote!(|| #block)
    };

    quote!( ::proc_macro_error::entry_point(#closure, #is_proc_macro_hack) )
}

// FIXME:
// proc_macro::TokenStream does not implement UnwindSafe until 1.37.0.
// Considering this is the closure's return type the unwind safety check would fail
// for virtually every closure possible, the check is meaningless.
#[cfg(always_assert_unwind)]
fn gen_body(block: Block, settings: Settings) -> proc_macro2::TokenStream {
    let is_proc_macro_hack = settings.is_set(ProcMacroHack);
    let closure = quote!(::std::panic::AssertUnwindSafe(|| #block ));
    quote!( ::proc_macro_error::entry_point(#closure, #is_proc_macro_hack) )
}

fn detect_proc_macro_hack(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path.is_ident("proc_macro_hack"))
}

fn is_proc_macro(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path.is_ident("proc_macro")
            || attr.path.is_ident("proc_macro_derive")
            || attr.path.is_ident("proc_macro_attribute")
    })
}
