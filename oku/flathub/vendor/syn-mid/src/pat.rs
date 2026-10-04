use syn::{punctuated::Punctuated, token, Attribute, Ident, Member, Path, Token, Type};

ast_enum_of_structs! {
    /// A pattern in a local binding, function signature, match expression, or
    /// various other places.
    ///
    /// # Syntax tree enum
    ///
    /// This type is a [syntax tree enum].
    ///
    /// [syntax tree enum]: https://docs.rs/syn/1/syn/enum.Expr.html#syntax-tree-enums
    pub enum Pat {
        /// A pattern that binds a new variable: `ref mut binding @ SUBPATTERN`.
        Ident(PatIdent),

        /// A path pattern like `Color::Red`.
        Path(PatPath),

        /// A reference pattern: `&mut var`.
        Reference(PatReference),

        /// A struct or struct variant pattern: `Variant { x, y, .. }`.
        Struct(PatStruct),

        /// A tuple pattern: `(a, b)`.
        Tuple(PatTuple),

        /// A tuple struct or tuple variant pattern: `Variant(x, y, .., z)`.
        TupleStruct(PatTupleStruct),

        /// A type ascription pattern: `foo: f64`.
        Type(PatType),

        /// A pattern that matches any value: `_`.
        Wild(PatWild),

        #[doc(hidden)]
        __NonExhaustive,
    }
}

ast_struct! {
    /// A pattern that binds a new variable: `ref mut binding @ SUBPATTERN`.
    pub struct PatIdent {
        pub attrs: Vec<Attribute>,
        pub by_ref: Option<Token![ref]>,
        pub mutability: Option<Token![mut]>,
        pub ident: Ident,
    }
}

ast_struct! {
    /// A path pattern like `Color::Red`.
    pub struct PatPath {
        pub attrs: Vec<Attribute>,
        pub path: Path,
    }
}

ast_struct! {
    /// A reference pattern: `&mut var`.
    pub struct PatReference {
        pub attrs: Vec<Attribute>,
        pub and_token: Token![&],
        pub mutability: Option<Token![mut]>,
        pub pat: Box<Pat>,
    }
}

ast_struct! {
    /// A struct or struct variant pattern: `Variant { x, y, .. }`.
    pub struct PatStruct {
        pub attrs: Vec<Attribute>,
        pub path: Path,
        pub brace_token: token::Brace,
        pub fields: Punctuated<FieldPat, Token![,]>,
        pub dot2_token: Option<Token![..]>,
    }
}

ast_struct! {
    /// A tuple pattern: `(a, b)`.
    pub struct PatTuple {
        pub attrs: Vec<Attribute>,
        pub paren_token: token::Paren,
        pub elems: Punctuated<Pat, Token![,]>,
    }
}

ast_struct! {
    /// A tuple struct or tuple variant pattern: `Variant(x, y, .., z)`.
    pub struct PatTupleStruct {
        pub attrs: Vec<Attribute>,
        pub path: Path,
        pub pat: PatTuple,
    }
}

ast_struct! {
    /// A type ascription pattern: `foo: f64`.
    pub struct PatType {
        pub attrs: Vec<Attribute>,
        pub pat: Box<Pat>,
        pub colon_token: Token![:],
        pub ty: Box<Type>,
    }
}

ast_struct! {
    /// A pattern that matches any value: `_`.
    pub struct PatWild {
        pub attrs: Vec<Attribute>,
        pub underscore_token: Token![_],
    }
}

ast_struct! {
    /// A single field in a struct pattern.
    ///
    /// Patterns like the fields of Foo `{ x, ref y, ref mut z }` are treated
    /// the same as `x: x, y: ref y, z: ref mut z` but there is no colon token.
    pub struct FieldPat {
        pub attrs: Vec<Attribute>,
        pub member: Member,
        pub colon_token: Option<Token![:]>,
        pub pat: Box<Pat>,
    }
}

mod parsing {
    use syn::{
        braced,
        ext::IdentExt,
        parenthesized,
        parse::{Parse, ParseStream, Result},
        punctuated::Punctuated,
        token, Attribute, Ident, Member, Path, Token,
    };

    use super::{
        FieldPat, Pat, PatIdent, PatPath, PatReference, PatStruct, PatTuple, PatTupleStruct,
        PatWild,
    };
    use crate::path;

    impl Parse for Pat {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            let lookahead = input.lookahead1();
            if {
                let ahead = input.fork();
                ahead.parse::<Option<Ident>>()?.is_some()
                    && (ahead.peek(Token![::])
                        || ahead.peek(token::Brace)
                        || ahead.peek(token::Paren))
            } || {
                let ahead = input.fork();
                ahead.parse::<Option<Token![self]>>()?.is_some() && ahead.peek(Token![::])
            } || lookahead.peek(Token![::])
                || lookahead.peek(Token![<])
                || input.peek(Token![Self])
                || input.peek(Token![super])
                || input.peek(Token![extern])
                || input.peek(Token![crate])
            {
                pat_path_or_struct(input)
            } else if lookahead.peek(Token![_]) {
                input.call(pat_wild).map(Pat::Wild)
            } else if lookahead.peek(Token![ref])
                || lookahead.peek(Token![mut])
                || input.peek(Token![self])
                || input.peek(Ident)
            {
                input.call(pat_ident).map(Pat::Ident)
            } else if lookahead.peek(Token![&]) {
                input.call(pat_reference).map(Pat::Reference)
            } else if lookahead.peek(token::Paren) {
                input.call(pat_tuple).map(Pat::Tuple)
            } else {
                Err(lookahead.error())
            }
        }
    }

    fn pat_path_or_struct(input: ParseStream<'_>) -> Result<Pat> {
        let path = path::parse_path(input)?;

        if input.peek(token::Brace) {
            pat_struct(input, path).map(Pat::Struct)
        } else if input.peek(token::Paren) {
            pat_tuple_struct(input, path).map(Pat::TupleStruct)
        } else {
            Ok(Pat::Path(PatPath { attrs: Vec::new(), path }))
        }
    }

    fn pat_wild(input: ParseStream<'_>) -> Result<PatWild> {
        Ok(PatWild { attrs: Vec::new(), underscore_token: input.parse()? })
    }

    fn pat_ident(input: ParseStream<'_>) -> Result<PatIdent> {
        Ok(PatIdent {
            attrs: Vec::new(),
            by_ref: input.parse()?,
            mutability: input.parse()?,
            ident: input.call(Ident::parse_any)?,
        })
    }

    fn pat_tuple_struct(input: ParseStream<'_>, path: Path) -> Result<PatTupleStruct> {
        Ok(PatTupleStruct { attrs: Vec::new(), path, pat: input.call(pat_tuple)? })
    }

    fn pat_struct(input: ParseStream<'_>, path: Path) -> Result<PatStruct> {
        let content;
        let brace_token = braced!(content in input);

        let mut fields = Punctuated::new();
        while !content.is_empty() && !content.peek(Token![..]) {
            let value = content.call(field_pat)?;
            fields.push_value(value);
            if content.is_empty() {
                break;
            }
            let punct: Token![,] = content.parse()?;
            fields.push_punct(punct);
        }

        let dot2_token = if fields.empty_or_trailing() && content.peek(Token![..]) {
            Some(content.parse()?)
        } else {
            None
        };

        Ok(PatStruct { attrs: Vec::new(), path, brace_token, fields, dot2_token })
    }

    fn field_pat(input: ParseStream<'_>) -> Result<FieldPat> {
        let attrs = input.call(Attribute::parse_outer)?;
        let boxed: Option<Token![box]> = input.parse()?;
        let by_ref: Option<Token![ref]> = input.parse()?;
        let mutability: Option<Token![mut]> = input.parse()?;
        let member: Member = input.parse()?;

        if boxed.is_none() && by_ref.is_none() && mutability.is_none() && input.peek(Token![:])
            || is_unnamed(&member)
        {
            return Ok(FieldPat {
                attrs,
                member,
                colon_token: input.parse()?,
                pat: input.parse()?,
            });
        }

        let ident = match member {
            Member::Named(ident) => ident,
            Member::Unnamed(_) => unreachable!(),
        };

        let pat =
            Pat::Ident(PatIdent { attrs: Vec::new(), by_ref, mutability, ident: ident.clone() });

        Ok(FieldPat { attrs, member: Member::Named(ident), colon_token: None, pat: Box::new(pat) })
    }

    fn pat_tuple(input: ParseStream<'_>) -> Result<PatTuple> {
        let content;
        let paren_token = parenthesized!(content in input);

        let mut elems = Punctuated::new();
        while !content.is_empty() {
            let value: Pat = content.parse()?;
            elems.push_value(value);
            if content.is_empty() {
                break;
            }
            let punct = content.parse()?;
            elems.push_punct(punct);
        }

        Ok(PatTuple { attrs: Vec::new(), paren_token, elems })
    }

    fn pat_reference(input: ParseStream<'_>) -> Result<PatReference> {
        Ok(PatReference {
            attrs: Vec::new(),
            and_token: input.parse()?,
            mutability: input.parse()?,
            pat: input.parse()?,
        })
    }

    fn is_unnamed(member: &Member) -> bool {
        match member {
            Member::Named(_) => false,
            Member::Unnamed(_) => true,
        }
    }
}

mod printing {
    use proc_macro2::TokenStream;
    use quote::{ToTokens, TokenStreamExt};
    use syn::Token;

    use super::{
        FieldPat, PatIdent, PatPath, PatReference, PatStruct, PatTuple, PatTupleStruct, PatType,
        PatWild,
    };

    impl ToTokens for PatWild {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.underscore_token.to_tokens(tokens);
        }
    }

    impl ToTokens for PatIdent {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.by_ref.to_tokens(tokens);
            self.mutability.to_tokens(tokens);
            self.ident.to_tokens(tokens);
        }
    }

    impl ToTokens for PatStruct {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.path.to_tokens(tokens);
            self.brace_token.surround(tokens, |tokens| {
                self.fields.to_tokens(tokens);
                // Note: We need a comma before the dot2 token if it is present.
                if !self.fields.empty_or_trailing() && self.dot2_token.is_some() {
                    <Token![,]>::default().to_tokens(tokens);
                }
                self.dot2_token.to_tokens(tokens);
            });
        }
    }

    impl ToTokens for PatTupleStruct {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.path.to_tokens(tokens);
            self.pat.to_tokens(tokens);
        }
    }

    impl ToTokens for PatType {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append_all(&self.attrs);
            self.pat.to_tokens(tokens);
            self.colon_token.to_tokens(tokens);
            self.ty.to_tokens(tokens);
        }
    }

    impl ToTokens for PatPath {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.path.to_tokens(tokens);
        }
    }

    impl ToTokens for PatTuple {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.paren_token.surround(tokens, |tokens| {
                self.elems.to_tokens(tokens);
            });
        }
    }

    impl ToTokens for PatReference {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.and_token.to_tokens(tokens);
            self.mutability.to_tokens(tokens);
            self.pat.to_tokens(tokens);
        }
    }

    impl ToTokens for FieldPat {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            if let Some(colon_token) = &self.colon_token {
                self.member.to_tokens(tokens);
                colon_token.to_tokens(tokens);
            }
            self.pat.to_tokens(tokens);
        }
    }
}
