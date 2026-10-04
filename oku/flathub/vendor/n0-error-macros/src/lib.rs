use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, Attribute, DeriveInput, Expr, Field,
    Fields, FieldsNamed, Ident,
};

/// Attribute macro to expand error enums or structs.
///
/// This macro can be applied to enums and structs and the syntax `#[stack_error(args)]`,
/// where `args` is a comma-seperated list of arguments.
/// * **`add_meta`** adds a `n0_error::Meta` field to the struct or each each enum variant. The `Meta`
///   struct contains call-site location metadata for the error.
///   - If the item has named fields, the field will added with name `meta`
///   - If the item is a tuple, the field will be added as the last field and marked with `#[error(meta)]`
///   - It the item is a unit, it will be altered to named fields, and the field
///     will be added with name `meta`
/// * **`derive`** will add `#[derive(StackError)]`. See the docs for the [`StackError`] for details.
/// * The item-level options of [`StackError`] derive macro (namely `from_sources` and `std_sources`)
///   can be set `stack_error` as well. They will be expanded to an `#[error]` attribute on the item.
///   See the documentation for [`StackError`] for details.
///
/// This is an attribute macro because it *modifies* its item by adding the `meta` field,
/// which derive macros cannot.
///
/// ## Example
///
/// ```ignore
/// #[stack_error(derive, add_meta, from_sources)]
/// enum MyError {
///     #[error("io failed")]
///     Io { source: std::io::Error },
///     #[error("remote failed with {_0}")]
///     RemoteErrorCode(u32),
/// }
/// ```
/// expands to
/// ```ignore
/// #[derive(n0_error::StackError)]
/// #[error(from_sources)]
/// enum MyError {
///     #[error("io failed")]
///     Io {
///         source: std::io::Error,
///         meta: n0_error::Meta,
///     },
///     #[error("remote failed with {_0}")]
///     RemoteErrorCode(u32, #[error(meta)] n0_error::Meta),
/// }
/// ```
#[proc_macro_attribute]
pub fn stack_error(args: TokenStream, item: TokenStream) -> TokenStream {
    match stack_error_inner(args, parse_macro_input!(item as syn::Item)) {
        Err(err) => err.to_compile_error().into(),
        Ok(tokens) => tokens.into(),
    }
}

fn stack_error_inner(
    args: TokenStream,
    mut input: syn::Item,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let args: StackErrAttrArgs = syn::parse(args.clone())?;
    match &mut input {
        syn::Item::Enum(item) => {
            if args.add_meta {
                for variant in item.variants.iter_mut() {
                    add_meta_field(&mut variant.fields);
                }
            }
            modify_attrs(&args, &mut item.attrs)?;
            Ok(quote! { #item })
        }
        syn::Item::Struct(item) => {
            if args.add_meta {
                add_meta_field(&mut item.fields);
            }
            modify_attrs(&args, &mut item.attrs)?;
            Ok(quote! { #item })
        }
        _ => Err(err(
            &input,
            "#[stack_error] only supports enums and structs",
        )),
    }
}

fn modify_attrs(args: &StackErrAttrArgs, attrs: &mut Vec<Attribute>) -> Result<(), syn::Error> {
    attrs.retain(|attr| !attr.path().is_ident("stackerr"));
    if args.derive {
        attrs.insert(0, parse_quote!(#[derive(::n0_error::StackError)]));
    }
    let error_args: Vec<_> = [
        args.from_sources.then(|| quote!(from_sources)),
        args.std_sources.then(|| quote!(std_sources)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !error_args.is_empty() {
        attrs.push(parse_quote!(#[error(#(#error_args),*)]))
    }

    Ok(())
}

fn add_meta_field(fields: &mut Fields) {
    let doc = "Captured call-site metadata";
    match fields {
        Fields::Named(fields) => {
            let field: syn::Field = parse_quote! { #[doc = #doc] meta: ::n0_error::Meta };
            fields.named.push(field);
        }
        Fields::Unit => {
            let mut named = FieldsNamed {
                brace_token: Default::default(),
                named: Default::default(),
            };
            let field: syn::Field = parse_quote! { #[doc = #doc] meta: ::n0_error::Meta };
            named.named.push(field);
            *fields = Fields::Named(named);
        }
        Fields::Unnamed(fields) => {
            let field: syn::Field = parse_quote! { #[doc = #doc] #[error(meta)] ::n0_error::Meta };
            fields.unnamed.push(field);
        }
    }
}

/// Derive macro that implements `StackError`, `Display`, `Debug` and `std::error::Error`
/// and generates `From<T>` impls for fields/variants configured via `#[error(..)]`.
/// Derive macro for stack errors.
///
/// This derive macro can be applied to structs and enums. Unit, tuple and named-field variants
/// are supported equally.
///
/// The macro will expand to implementations of `StackError`, [`Display`], [`Debug`] and [`Error`].
/// It will also add [`From`] impls for fields configured via the `error` attribute.
///
/// Items with the derive macro applied accept an `#[error(args)]` attribute, where `args` is a comma-separated
/// list of arguments. The supported arguments vary by on the kind of item:
///
/// * on enums:
///   - `#[error(from_sources)]`: Creates `From` impls for the `source` types of all variants. Will fail to compile if multiple sources have the same type.
///   - `#[error(std_sources)]`: Defaults all sources to be std errors instead of stack errors.
/// * on structs and enum variants:
///   - `#[error("format {field}: {}", a + b)]`: Sets the display formatting. You can refer to named fields by their names, and to tuple fields by `_0`, `_1` etc.
///   - `#[error(transparent)]`: Directly forwards the display implementation to the error source, and omits the outer error in the source chain when reporting errors.
/// * on fields:
///   - `#[error(from)]`: Creates a `From` impl for the field's type to the error type.
///   - `#[error(source)]`: The error's `source` method returns a reference to whatever field is named `source`, or
///     has the `source` attribute set.
///   - `#[error(std_err)]`: *Only on on `source` fields.* Marks the error as a `std` error.
///     Source fields not marked as `std_err` need to implement `StackError`.
///   - `#[error(stack_error)]`: *Only on `source` fields.* Marks the error as a `StackError`. This is the default unless `std_sources` is set on the top-level item.
///   - `#[error(meta)]: Sets a field as the `meta` field for this error or variant. Must have type [`::n0_error::Error`]
///
/// [`Display`]: std::fmt::Display
/// [`Debug`]: std::fmt::Debug
/// [`Error`]: std::error::Error.
#[proc_macro_derive(StackError, attributes(error))]
pub fn derive_stack_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match derive_error_inner(input) {
        Err(err) => err.to_compile_error().into(),
        Ok(tokens) => tokens.into(),
    }
}

fn derive_error_inner(input: DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        syn::Data::Enum(item) => {
            let args = EnumAttrArgs::from_attributes(&input.attrs)?;
            let infos = item
                .variants
                .iter()
                .map(|v| {
                    let ident = VariantIdent::Variant(&input.ident, &v.ident);
                    VariantInfo::parse(ident, &v.fields, &v.attrs, &args)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(generate_enum_impls(&input.ident, &input.generics, infos))
        }
        syn::Data::Struct(item) => {
            let ident = VariantIdent::Struct(&input.ident);
            let info = VariantInfo::parse(ident, &item.fields, &input.attrs, &Default::default())?;
            Ok(generate_struct_impl(&input.ident, &input.generics, info))
        }
        _ => Err(err(
            &input,
            "#[derive(StackError)] only supports enums or structs",
        )),
    }
}

struct SourceField<'a> {
    kind: SourceKind,
    field: FieldInfo<'a>,
}

impl<'a> SourceField<'a> {
    fn expr_error_ref(&self, expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self.kind {
            SourceKind::Std => quote! { Some(::n0_error::ErrorRef::std(#expr)) },
            SourceKind::Stack => quote! { Some(::n0_error::ErrorRef::stack(#expr)) },
        }
    }

    fn expr_error_std(&self, expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self.kind {
            SourceKind::Std => quote! { Some(#expr as &dyn ::std::error::Error) },
            SourceKind::Stack => quote! { Some(::n0_error::StackError::as_std(#expr)) },
        }
    }
}

enum SourceKind {
    Stack,
    Std,
}

#[derive(Default, Clone, Copy)]
struct StackErrAttrArgs {
    add_meta: bool,
    derive: bool,
    from_sources: bool,
    std_sources: bool,
}

impl syn::parse::Parse for StackErrAttrArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "add_meta" => out.add_meta = true,
                "derive" => out.derive = true,
                "from_sources" => out.from_sources = true,
                "std_sources" => out.std_sources = true,
                other => Err(err(
                    ident,
                    format!("unknown stack_error option `{}`", other),
                ))?,
            }
            if input.peek(syn::Token![,]) {
                let _ = input.parse::<syn::Token![,]>()?;
            }
        }
        Ok(out)
    }
}

#[derive(Default, Clone, Copy)]
struct EnumAttrArgs {
    from_sources: bool,
    std_sources: bool,
}

impl EnumAttrArgs {
    fn from_attributes(attrs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut out = Self::default();
        for ident in parse_error_attr_as_idents(attrs)? {
            match ident.to_string().as_str() {
                "from_sources" => out.from_sources = true,
                "std_sources" => out.std_sources = true,
                _ => Err(err(
                    ident,
                    "Invalid argument for the `error` attribute on fields",
                ))?,
            }
        }
        Ok(out)
    }
}

#[derive(Default, Clone, Copy)]
struct FieldAttrArgs {
    source: bool,
    from: bool,
    std_err: bool,
    stack_err: bool,
    meta: bool,
}

impl FieldAttrArgs {
    fn from_attributes(attrs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut out = Self::default();
        for ident in parse_error_attr_as_idents(attrs)? {
            match ident.to_string().as_str() {
                "source" => out.source = true,
                "from" => out.from = true,
                "std_err" => out.std_err = true,
                "stack_err" => out.stack_err = true,
                "meta" => out.meta = true,
                _ => Err(err(
                    ident,
                    "Invalid argument for the `error` attribute on fields",
                ))?,
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy)]
enum VariantIdent<'a> {
    Struct(&'a Ident),
    Variant(&'a Ident, &'a Ident),
}

impl<'a> VariantIdent<'a> {
    fn self_ident(&self) -> proc_macro2::TokenStream {
        match self {
            VariantIdent::Struct(_) => quote!(Self),
            VariantIdent::Variant(_, variant) => quote!(Self::#variant),
        }
    }
    fn item_ident(&self) -> &Ident {
        match self {
            VariantIdent::Struct(ident) => ident,
            VariantIdent::Variant(ident, _) => ident,
        }
    }

    fn inner(&self) -> &Ident {
        match self {
            VariantIdent::Struct(ident) => ident,
            VariantIdent::Variant(_, ident) => ident,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Named,
    Unit,
    Tuple,
}

struct VariantInfo<'a> {
    ident: VariantIdent<'a>,
    fields: Vec<FieldInfo<'a>>,
    kind: Kind,
    display: DisplayArgs,
    source: Option<SourceField<'a>>,
    /// The field that is used for From<..> impls
    from: Option<FieldInfo<'a>>,
    /// The meta field if present.
    meta: Option<FieldInfo<'a>>,
}

impl<'a> VariantInfo<'a> {
    fn parse(
        ident: VariantIdent<'a>,
        fields: &'a Fields,
        attrs: &[Attribute],
        args: &EnumAttrArgs,
    ) -> Result<VariantInfo<'a>, syn::Error> {
        let display = DisplayArgs::parse(attrs)?;
        let (kind, fields): (Kind, Vec<FieldInfo>) = match fields {
            Fields::Named(ref fields) => (
                Kind::Named,
                fields
                    .named
                    .iter()
                    .map(FieldInfo::from_named)
                    .collect::<Result<_, _>>()?,
            ),
            Fields::Unit => (Kind::Unit, Vec::new()),
            Fields::Unnamed(ref fields) => (
                Kind::Tuple,
                fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, f)| FieldInfo::from_unnamed(i, f))
                    .collect::<Result<_, _>>()?,
            ),
        };

        if fields.iter().filter(|f| f.args.source).count() > 1 {
            return Err(err(
                ident.inner(),
                "Only one field per variant may have #[error(source)]",
            ));
        }
        let source_field = fields
            .iter()
            .find(|f| f.args.source)
            .or_else(|| match kind {
                Kind::Named => fields
                    .iter()
                    .find(|f| matches!(f.ident, FieldIdent::Named(name) if name == "source")),
                Kind::Tuple if display.is_transparent() => fields.first(),
                _ => None,
            });

        if display.is_transparent() && source_field.is_none() {
            return Err(err(
                ident.inner(),
                "Variants with #[error(transparent)] require a source field",
            ));
        }

        // Determine source kind and optional From based on field options and top-level switches
        let source = source_field.as_ref().map(|field| {
            let kind = if field.args.std_err || (args.std_sources && !field.args.stack_err) {
                SourceKind::Std
            } else {
                SourceKind::Stack
            };
            SourceField {
                kind,
                field: (*field).clone(),
            }
        });

        let from_field = fields
            .iter()
            .find(|f| f.args.from)
            .or_else(|| args.from_sources.then_some(source_field).flatten());
        let meta_field = fields.iter().find(|f| f.is_meta()).cloned();
        Ok(VariantInfo {
            ident,
            kind,
            display,
            from: from_field.cloned(),
            meta: meta_field,
            source,
            fields,
        })
    }

    fn transparent(&self) -> Option<&FieldInfo<'_>> {
        match self.display {
            DisplayArgs::Transparent => self.source.as_ref().map(|s| &s.field),
            _ => None,
        }
    }

    fn spread_field(&self, field: &FieldInfo<'a>, bind: &Ident) -> proc_macro2::TokenStream {
        let slf = self.ident.self_ident();
        match field.ident {
            FieldIdent::Named(ident) => {
                quote! { #slf { #ident: #bind, .. } }
            }
            FieldIdent::Unnamed(_) => {
                let pats = self.fields.iter().map(|f| {
                    if f.ident == field.ident {
                        quote!(#bind)
                    } else {
                        quote!(_)
                    }
                });
                quote! { #slf ( #(#pats),* ) }
            }
        }
    }

    fn spread_empty(&self) -> proc_macro2::TokenStream {
        let slf = self.ident.self_ident();
        match self.kind {
            Kind::Unit => quote! { #slf },
            Kind::Named => quote! { #slf { .. } },
            Kind::Tuple => {
                let pats = std::iter::repeat_n(quote! { _ }, self.fields.len());
                quote! { #slf ( #(#pats),* ) }
            }
        }
    }

    fn spread_all(&self) -> proc_macro2::TokenStream {
        let binds = self.fields.iter().map(|f| f.ident.as_ident());
        self.spread(binds.map(|i| quote!(#i)))
    }

    fn spread(
        &self,
        fields: impl Iterator<Item = proc_macro2::TokenStream>,
    ) -> proc_macro2::TokenStream {
        let slf = self.ident.self_ident();
        match self.kind {
            Kind::Unit => quote! { #slf },
            Kind::Named => quote! { #slf { #(#fields),* } },
            Kind::Tuple => quote! { #slf ( #(#fields),* ) },
        }
    }

    fn generate_from_impl(&self) -> Option<(&syn::Type, proc_macro2::TokenStream)> {
        self.from.as_ref().map(|from_field| {
            let ty = &from_field.field.ty;
            let fields = self.fields.iter().map(|field| {
                if field.ident == from_field.ident {
                    field.ident.init(quote!(source))
                } else if field.is_meta() {
                    field.ident.init(quote!(::n0_error::Meta::new()))
                } else {
                    field.ident.init(quote!(::std::default::Default::default()))
                }
            });
            let construct = self.spread(fields);
            (ty, construct)
        })
    }
}

#[derive(Clone)]
struct FieldInfo<'a> {
    field: &'a Field,
    args: FieldAttrArgs,
    ident: FieldIdent<'a>,
}

impl<'a> FieldInfo<'a> {
    fn from_named(field: &'a Field) -> Result<Self, syn::Error> {
        Ok(Self {
            args: FieldAttrArgs::from_attributes(&field.attrs)?,
            ident: FieldIdent::Named(field.ident.as_ref().unwrap()),
            field,
        })
    }
    fn from_unnamed(index: usize, field: &'a Field) -> Result<Self, syn::Error> {
        Ok(Self {
            args: FieldAttrArgs::from_attributes(&field.attrs)?,
            ident: FieldIdent::Unnamed(index),
            field,
        })
    }

    fn is_meta(&self) -> bool {
        self.args.meta || matches!(self.ident, FieldIdent::Named(ident) if ident == "meta")
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FieldIdent<'a> {
    Named(&'a Ident),
    Unnamed(usize),
}

impl<'a> FieldIdent<'a> {
    fn named(&self) -> Option<&'a Ident> {
        match self {
            FieldIdent::Named(ident) => Some(ident),
            _ => None,
        }
    }

    fn as_ident(&self) -> Ident {
        match self {
            FieldIdent::Named(ident) => (*ident).clone(),
            FieldIdent::Unnamed(i) => syn::Ident::new(&format!("_{}", i), Span::call_site()),
        }
    }

    fn self_expr(&self) -> proc_macro2::TokenStream {
        match self {
            FieldIdent::Named(ident) => quote!(&self.#ident),
            FieldIdent::Unnamed(i) => {
                let idx = syn::Index::from(*i);
                quote!(&self.#idx)
            }
        }
    }

    fn init(&self, expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            FieldIdent::Named(ident) => quote!(#ident: #expr),
            FieldIdent::Unnamed(_) => quote!(#expr),
        }
    }
}

fn generate_enum_impls(
    enum_ident: &Ident,
    generics: &syn::Generics,
    variants: Vec<VariantInfo>,
) -> proc_macro2::TokenStream {
    let match_meta_arms = variants.iter().map(|vi| {
        if let Some(meta_field) = &vi.meta {
            let bind = syn::Ident::new("__meta", Span::call_site());
            let pat = vi.spread_field(meta_field, &bind);
            quote! { #pat => Some(&#bind) }
        } else {
            let pat = vi.spread_empty();
            quote! { #pat => None }
        }
    });

    let match_source_arms = variants.iter().map(|vi| match &vi.source {
        Some(src) => {
            let bind = syn::Ident::new("__source", Span::call_site());
            let pat = vi.spread_field(&src.field, &bind);
            let expr = src.expr_error_ref(quote!(#bind));
            quote! { #pat => #expr, }
        }
        None => {
            let pat = vi.spread_empty();
            quote! { #pat => None, }
        }
    });

    let match_std_source_arms = variants.iter().map(|vi| match &vi.source {
        Some(src) => {
            let bind = syn::Ident::new("__source", Span::call_site());
            let pat = vi.spread_field(&src.field, &bind);
            let expr = src.expr_error_std(quote!(#bind));
            quote! { #pat => #expr, }
        }
        None => {
            let pat = vi.spread_empty();
            quote! { #pat => None, }
        }
    });

    let match_transparent_arms = variants.iter().map(|vi| {
        let value = vi.transparent().is_some();
        let pat = vi.spread_empty();
        quote! { #pat => #value }
    });

    let match_fmt_message_arms = variants.iter().map(|vi| match &vi.display {
        DisplayArgs::Format(expr) => {
            let pat = vi.spread_all();
            quote! {
                #[allow(unused)]
                #pat => { #expr }
            }
        }
        DisplayArgs::Default | DisplayArgs::Transparent => {
            let text = format!("{}::{}", vi.ident.item_ident(), vi.ident.inner());
            let pat = vi.spread_empty();
            quote! { #pat => write!(f, #text) }
        }
    });

    let match_debug_arms = variants.iter().map(|vi| {
        let v_name = vi.ident.inner().to_string();
        let binds = vi.fields.iter().map(|f| f.ident.as_ident());
        let pat = vi.spread_all();
        let labels: Vec<String> = vi
            .fields
            .iter()
            .map(|f| match f.ident {
                FieldIdent::Named(id) => id.to_string(),
                FieldIdent::Unnamed(i) => i.to_string(),
            })
            .collect();
        quote! {
            #pat => {
                let mut dbg = f.debug_struct(#v_name);
                #( dbg.field(#labels, &#binds); )*; dbg.finish()?;
            }
        }
    });

    // From impls for variant fields marked with #[from]
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_impls = variants.iter().filter_map(|vi| vi.generate_from_impl()).map(|(ty, construct)| {
        quote! {
            impl #impl_generics ::core::convert::From<#ty> for #enum_ident #ty_generics #where_clause {
                #[allow(unreachable_code)]
                #[track_caller]
                fn from(source: #ty) -> Self {
                    #construct
                }
            }
        }
    });

    quote! {
        impl #impl_generics ::n0_error::StackError for #enum_ident #ty_generics #where_clause {
            fn as_std(&self) -> &(dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync + 'static) {
                self
            }
            fn into_std(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn std::error::Error + ::std::marker::Send + ::std::marker::Sync> {
                self
            }
            fn as_dyn(&self) -> &(dyn ::n0_error::StackError) {
                self
            }
            fn fmt_message(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #( #match_fmt_message_arms, )*
                }
            }
            fn meta(&self) -> Option<&::n0_error::Meta> {
                match self {
                    #( #match_meta_arms, )*
                }
            }
            fn source(&self) -> Option<::n0_error::ErrorRef<'_>> {
                match self {
                    #( #match_source_arms )*
                }
            }
            fn is_transparent(&self) -> bool {
                match self {
                    #( #match_transparent_arms, )*
                }
            }
        }

        impl #impl_generics ::core::convert::From<#enum_ident #ty_generics> for ::n0_error::AnyError #where_clause {
            fn from(value: #enum_ident #ty_generics) -> ::n0_error::AnyError {
                ::n0_error::AnyError::from_stack(value)
            }
        }

        impl #impl_generics ::std::fmt::Display for #enum_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let sources = f.alternate().then_some(::n0_error::SourceFormat::OneLine);
                write!(f, "{}", ::n0_error::StackError::report(self).sources(sources))
            }
        }

        impl #impl_generics ::std::fmt::Debug for #enum_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                if f.alternate() {
                    match self {
                        #(#match_debug_arms)*
                    }
                } else {
                    write!(f, "{}", ::n0_error::StackError::report(self).full())?;
                }
                Ok(())
            }
        }

        impl #impl_generics ::std::error::Error for #enum_ident #ty_generics #where_clause {
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    #( #match_std_source_arms )*
                }
            }
        }
        #( #from_impls )*
    }
}

fn generate_struct_impl(
    item_ident: &Ident,
    generics: &syn::Generics,
    info: VariantInfo,
) -> proc_macro2::TokenStream {
    let constructor = {
        let params = info.fields.iter().filter(|f| !f.is_meta()).map(|f| {
            let ty = &f.field.ty;
            let ident = f.ident.as_ident();
            quote! { #ident: #ty }
        });
        let fields = info.fields.iter().map(|f| {
            if f.is_meta() {
                f.ident.init(quote!(::n0_error::Meta::new()))
            } else {
                let ident = f.ident.as_ident();
                quote!(#ident)
            }
        });
        let construct = info.spread(fields);
        let doc = format!("Creates a new [`{}`] error.", item_ident);
        quote! {
            #[doc = #doc]
            #[allow(unreachable_code)]
            #[track_caller]
            pub fn new(#(#params),*) -> Self { #construct }
        }
    };
    let get_meta = if let Some(field) = &info.meta {
        let get_expr = field.ident.self_expr();
        quote! { Some(&#get_expr) }
    } else {
        quote! { None }
    };

    let get_error_source = match &info.source {
        Some(src) => src.expr_error_ref(src.field.ident.self_expr()),
        None => quote! { None },
    };

    let get_std_source = match &info.source {
        Some(src) => src.expr_error_std(src.field.ident.self_expr()),
        None => quote! { None },
    };

    let is_transparent = info.transparent().is_some();

    let get_fmt_message = {
        match &info.display {
            DisplayArgs::Format(expr) => {
                let pat = info.spread_all();
                quote! {
                   #[allow(unused)]
                   let #pat = self;
                   #expr
                }
            }
            DisplayArgs::Default | DisplayArgs::Transparent => {
                let text = info.ident.item_ident().to_string();
                quote! { write!(f, #text) }
            }
        }
    };

    let get_debug = {
        let item_name = info.ident.item_ident().to_string();
        match info.kind {
            Kind::Unit => quote!(write!(f, #item_name)?;),
            Kind::Named => {
                let fields = info
                    .fields
                    .iter()
                    .filter_map(|f| f.ident.named())
                    .map(|ident| {
                        let ident_s = ident.to_string();
                        quote! { dbg.field(#ident_s, &self.#ident) }
                    });
                quote! {
                    let mut dbg = f.debug_struct(#item_name);
                    #(#fields);*;
                    dbg.finish()?;
                }
            }
            Kind::Tuple => {
                let binds = (0..info.fields.len()).map(syn::Index::from);
                quote! {
                    let mut dbg = f.debug_tuple(#item_name);
                    #( dbg.field(&self.#binds); )*;
                    dbg.finish()?;
                }
            }
        }
    };

    // From impls for fields marked with #[error(from)] (or inferred via from_sources)
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let from_impl = info.generate_from_impl().map(|(ty, construct)| {
        quote! {
            impl #impl_generics ::core::convert::From<#ty> for #item_ident #ty_generics #where_clause {
                #[allow(unreachable_code)]
                #[track_caller]
                fn from(source: #ty) -> Self { #construct }
            }
        }
    });

    quote! {
        impl #impl_generics #item_ident #ty_generics #where_clause {
            #constructor
        }

        impl #impl_generics ::n0_error::StackError for #item_ident #ty_generics #where_clause {
            fn as_std(&self) -> &(dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync + 'static) {
                self
            }
            fn into_std(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn std::error::Error + ::std::marker::Send + ::std::marker::Sync> {
                self
            }
            fn as_dyn(&self) -> &(dyn ::n0_error::StackError) {
                self
            }
            fn meta(&self) -> Option<&::n0_error::Meta> {
                #get_meta
            }
            fn source(&self) -> Option<::n0_error::ErrorRef<'_>> {
                #get_error_source
            }
            fn is_transparent(&self) -> bool {
                #is_transparent
            }
            fn fmt_message(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                #get_fmt_message
            }
        }


        impl #impl_generics ::core::convert::From<#item_ident #ty_generics> for ::n0_error::AnyError #where_clause {
            fn from(value: #item_ident #ty_generics) -> ::n0_error::AnyError {
                ::n0_error::AnyError::from_stack(value)
            }
        }

        impl #impl_generics ::std::fmt::Display for #item_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let sources = f.alternate().then_some(::n0_error::SourceFormat::OneLine);
                write!(f, "{}", ::n0_error::StackError::report(self).sources(sources))
            }
        }

        impl #impl_generics ::std::fmt::Debug for #item_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                if f.alternate() {
                    #get_debug
                } else {
                    write!(f, "{}", ::n0_error::StackError::report(self).full())?;
                }
                Ok(())
            }
        }

        impl #impl_generics ::std::error::Error for #item_ident #ty_generics #where_clause {
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
                #get_std_source
            }
        }

        #from_impl
    }
}

enum DisplayArgs {
    Default,
    Transparent,
    Format(proc_macro2::TokenStream),
}

impl DisplayArgs {
    fn is_transparent(&self) -> bool {
        matches!(self, Self::Transparent)
    }

    fn parse(attrs: &[Attribute]) -> Result<DisplayArgs, syn::Error> {
        // Only consider #[error(...)]
        let Some(attr) = attrs.iter().find(|a| a.path().is_ident("error")) else {
            return Ok(DisplayArgs::Default);
        };

        let args: Punctuated<Expr, syn::Token![,]> =
            attr.parse_args_with(Punctuated::<Expr, syn::Token![,]>::parse_terminated)?;

        if args.is_empty() {
            return Err(err(
                attr,
                "#[error(..)] requires arguments: a format string or `transparent`",
            ));
        }

        // #[error(transparent)]
        if args.len() == 1 {
            if let Expr::Path(p) = &args[0] {
                if p.path.is_ident("transparent") {
                    return Ok(DisplayArgs::Transparent);
                }
            }
        }

        // #[error("...", args...)]
        let mut args = args.into_iter();
        let first = args.next().unwrap();
        let fmt_lit = match first {
            Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => s,
            other => return Err(err(
                other,
                "first argument to #[error(\"...\")] must be a string literal, or use #[error(transparent)]",
            ))
        };

        let rest: Vec<Expr> = args.collect();
        Ok(DisplayArgs::Format(
            quote! { write!(f, #fmt_lit #(, #rest)* ) },
        ))
    }
}

fn err(ident: impl ToTokens, err: impl ToString) -> syn::Error {
    syn::Error::new_spanned(ident, err.to_string())
}

fn parse_error_attr_as_idents(attrs: &[Attribute]) -> Result<Vec<Ident>, syn::Error> {
    let mut out = vec![];
    for attr in attrs.iter().filter(|a| a.path().is_ident("error")) {
        let idents = attr.parse_args_with(|input: syn::parse::ParseStream<'_>| {
            let list: Punctuated<Ident, syn::Token![,]> =
                Punctuated::<Ident, syn::Token![,]>::parse_terminated(input)?;
            Ok(list.into_iter())
        })?;
        out.extend(idents);
    }
    Ok(out)
}
