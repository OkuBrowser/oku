use proc_macro2::TokenStream;
use syn::{
    punctuated::Punctuated, token, Abi, Attribute, Generics, Ident, Lifetime, ReturnType, Token,
    Variadic, Visibility,
};

use super::PatType;

ast_struct! {
    /// A free-standing function: `fn process(n: usize) -> Result<()> { ...
    /// }`.
    pub struct ItemFn {
        pub attrs: Vec<Attribute>,
        pub vis: Visibility,
        pub sig: Signature,
        pub block: Box<Block>,
    }
}

ast_struct! {
    /// A braced block containing Rust statements.
    pub struct Block {
        pub brace_token: token::Brace,
        /// Statements in a block
        pub stmts: TokenStream,
    }
}

ast_struct! {
    /// A function signature in a trait or implementation: `unsafe fn
    /// initialize(&self)`.
    pub struct Signature {
        pub constness: Option<Token![const]>,
        pub asyncness: Option<Token![async]>,
        pub unsafety: Option<Token![unsafe]>,
        pub abi: Option<Abi>,
        pub fn_token: Token![fn],
        pub ident: Ident,
        pub generics: Generics,
        pub paren_token: token::Paren,
        pub inputs: Punctuated<FnArg, Token![,]>,
        pub variadic: Option<Variadic>,
        pub output: ReturnType,
    }
}

ast_enum_of_structs! {
    /// An argument in a function signature: the `n: usize` in `fn f(n: usize)`.
    pub enum FnArg {
        /// The `self` argument of an associated method, whether taken by value
        /// or by reference.
        Receiver(Receiver),

        /// A function argument accepted by pattern and type.
        Typed(PatType),
    }
}

ast_struct! {
    /// The `self` argument of an associated method, whether taken by value
    /// or by reference.
    pub struct Receiver {
        pub attrs: Vec<Attribute>,
        pub reference: Option<(Token![&], Option<Lifetime>)>,
        pub mutability: Option<Token![mut]>,
        pub self_token: Token![self],
    }
}

mod parsing {
    use syn::{
        braced, parenthesized,
        parse::{discouraged::Speculative, Parse, ParseStream, Result},
        parse2, Abi, Attribute, Generics, Ident, ReturnType, Token, Type, Variadic, Visibility,
    };

    use super::{Block, FnArg, ItemFn, PatType, Receiver, Signature};

    impl Parse for Block {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            let content;
            Ok(Self { brace_token: braced!(content in input), stmts: content.parse()? })
        }
    }

    impl Parse for Signature {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            #[allow(clippy::trivially_copy_pass_by_ref)]
            fn get_variadic(input: &&FnArg) -> Option<Variadic> {
                if let FnArg::Typed(PatType { ty, .. }) = input {
                    if let Type::Verbatim(tokens) = &**ty {
                        if let Ok(dots) = parse2(tokens.clone()) {
                            return Some(Variadic { attrs: Vec::new(), dots });
                        }
                    }
                }
                None
            }

            let constness: Option<Token![const]> = input.parse()?;
            let asyncness: Option<Token![async]> = input.parse()?;
            let unsafety: Option<Token![unsafe]> = input.parse()?;
            let abi: Option<Abi> = input.parse()?;
            let fn_token: Token![fn] = input.parse()?;
            let ident: Ident = input.parse()?;
            let mut generics: Generics = input.parse()?;

            let content;
            let paren_token = parenthesized!(content in input);
            let inputs = content.parse_terminated(FnArg::parse)?;
            let variadic = inputs.last().as_ref().and_then(get_variadic);

            let output: ReturnType = input.parse()?;
            generics.where_clause = input.parse()?;

            Ok(Self {
                constness,
                asyncness,
                unsafety,
                abi,
                fn_token,
                ident,
                generics,
                paren_token,
                inputs,
                variadic,
                output,
            })
        }
    }

    impl Parse for ItemFn {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            let attrs = input.call(Attribute::parse_outer)?;
            let vis: Visibility = input.parse()?;
            let sig = input.parse()?;
            let block = input.parse()?;
            Ok(Self { attrs, vis, sig, block: Box::new(block) })
        }
    }

    impl Parse for FnArg {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            let attrs = input.call(Attribute::parse_outer)?;

            let ahead = input.fork();
            if let Ok(mut receiver) = ahead.parse::<Receiver>() {
                if !ahead.peek(Token![:]) {
                    input.advance_to(&ahead);
                    receiver.attrs = attrs;
                    return Ok(FnArg::Receiver(receiver));
                }
            }

            let mut typed = input.call(fn_arg_typed)?;
            typed.attrs = attrs;
            Ok(FnArg::Typed(typed))
        }
    }

    impl Parse for Receiver {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            Ok(Self {
                attrs: Vec::new(),
                reference: {
                    if input.peek(Token![&]) {
                        Some((input.parse()?, input.parse()?))
                    } else {
                        None
                    }
                },
                mutability: input.parse()?,
                self_token: input.parse()?,
            })
        }
    }

    fn fn_arg_typed(input: ParseStream<'_>) -> Result<PatType> {
        Ok(PatType {
            attrs: Vec::new(),
            pat: input.parse()?,
            colon_token: input.parse()?,
            ty: Box::new(input.parse()?),
        })
    }
}

mod printing {
    use proc_macro2::TokenStream;
    use quote::{ToTokens, TokenStreamExt};
    use syn::{punctuated::Punctuated, Token, Type};

    use super::{Block, FnArg, ItemFn, Receiver, Signature};

    impl ToTokens for Block {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.brace_token.surround(tokens, |tokens| {
                tokens.append_all(self.stmts.clone());
            });
        }
    }

    fn has_variadic(inputs: &Punctuated<FnArg, Token![,]>) -> bool {
        let last = match inputs.last() {
            Some(last) => last,
            None => return false,
        };

        let pat = match last {
            FnArg::Typed(pat) => pat,
            FnArg::Receiver(_) => return false,
        };

        let tokens = match &*pat.ty {
            Type::Verbatim(tokens) => tokens,
            _ => return false,
        };

        tokens.to_string() == "..."
    }

    impl ToTokens for Signature {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.constness.to_tokens(tokens);
            self.asyncness.to_tokens(tokens);
            self.unsafety.to_tokens(tokens);
            self.abi.to_tokens(tokens);
            self.fn_token.to_tokens(tokens);
            self.ident.to_tokens(tokens);
            self.generics.to_tokens(tokens);
            self.paren_token.surround(tokens, |tokens| {
                self.inputs.to_tokens(tokens);
                if self.variadic.is_some() && !has_variadic(&self.inputs) {
                    if !self.inputs.empty_or_trailing() {
                        <Token![,]>::default().to_tokens(tokens);
                    }
                    self.variadic.to_tokens(tokens);
                }
            });
            self.output.to_tokens(tokens);
            self.generics.where_clause.to_tokens(tokens);
        }
    }

    impl ToTokens for ItemFn {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append_all(&self.attrs);
            self.vis.to_tokens(tokens);
            self.sig.to_tokens(tokens);
            self.block.brace_token.surround(tokens, |tokens| {
                tokens.append_all(self.block.stmts.clone());
            });
        }
    }

    impl ToTokens for Receiver {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append_all(&self.attrs);
            if let Some((ampersand, lifetime)) = &self.reference {
                ampersand.to_tokens(tokens);
                lifetime.to_tokens(tokens);
            }
            self.mutability.to_tokens(tokens);
            self.self_token.to_tokens(tokens);
        }
    }
}
