use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, ExprLit, Fields, GenericArgument, Ident, Lit,
    LitStr, PathArguments, Type, meta::ParseNestedMeta, parse_macro_input, spanned::Spanned,
};

#[proc_macro_derive(MetricsGroup, attributes(metrics, default))]
pub fn derive_metrics_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut out = proc_macro2::TokenStream::new();
    out.extend(expand_metrics(&input).unwrap_or_else(Error::into_compile_error));
    out.extend(expand_iterable(&input).unwrap_or_else(Error::into_compile_error));
    out.into()
}

#[proc_macro_derive(Iterable)]
pub fn derive_iterable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let out = expand_iterable(&input).unwrap_or_else(Error::into_compile_error);
    out.into()
}

#[proc_macro_derive(MetricsGroupSet, attributes(metrics))]
pub fn derive_metrics_group_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut out = proc_macro2::TokenStream::new();
    out.extend(expand_metrics_group_set(&input).unwrap_or_else(Error::into_compile_error));
    out.into()
}

#[proc_macro_derive(EncodeLabelSet, attributes(label))]
pub fn derive_encode_label_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_encode_label_set(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Derives [`EncodeLabelValue`] for an enum with only unit variants.
///
/// Maps each variant to its name (snake_case by default). Use
/// `#[label(rename_all = "...")]` on the enum or `#[label(name = "...")]`
/// per-variant to customize.
#[proc_macro_derive(EncodeLabelValue, attributes(label))]
pub fn derive_encode_label_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_encode_label_value(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_iterable(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let (name, fields) = parse_named_struct(input)?;

    // Partition into scalars and families. `Family<_, _>` is detected by the
    // last path segment of the field type, so type aliases require an
    // explicit `#[metrics(family)]` override.
    let mut metrics = Vec::new();
    let mut families = Vec::new();
    for field in fields.iter() {
        let attr = parse_metrics_attr(&field.attrs)?;
        let info = field_info(field, attr)?;
        if info.is_family {
            families.push(info);
        } else {
            metrics.push(info);
        }
    }

    let metric_count = metrics.len();
    let family_count = families.len();

    let metric_arms = metrics.iter().enumerate().map(|(i, f)| {
        let (ident, ident_str, help) = (f.ident, &f.ident_str, &f.help);
        quote! {
            #i => Some(::iroh_metrics::MetricItem::new(#ident_str, #help, &self.#ident as &dyn ::iroh_metrics::Metric)),
        }
    });
    let family_arms = families.iter().enumerate().map(|(i, f)| {
        let (ident, ident_str, help) = (f.ident, &f.ident_str, &f.help);
        quote! {
            #i => Some(::iroh_metrics::FamilyItem::new(#ident_str, #help, &self.#ident as &dyn ::iroh_metrics::FamilyEncoder)),
        }
    });

    let family_impl = (family_count > 0).then(|| {
        quote! {
            fn family_field_count(&self) -> usize { #family_count }
            fn family_field_ref(&self, n: usize) -> Option<::iroh_metrics::FamilyItem<'_>> {
                match n {
                    #(#family_arms)*
                    _ => None,
                }
            }
        }
    });

    Ok(quote! {
        impl ::iroh_metrics::iterable::Iterable for #name {
            fn metric_field_count(&self) -> usize { #metric_count }
            fn metric_field_ref(&self, n: usize) -> Option<::iroh_metrics::MetricItem<'_>> {
                match n {
                    #(#metric_arms)*
                    _ => None,
                }
            }
            #family_impl
        }
    })
}

/// Per-field info pre-computed once for `expand_iterable`.
struct FieldInfo<'a> {
    ident: &'a Ident,
    ident_str: String,
    /// Help text: explicit `#[metrics(help = "...")]` > first doc line > field name.
    help: String,
    is_family: bool,
}

fn field_info<'a>(field: &'a syn::Field, attr: MetricsAttr) -> Result<FieldInfo<'a>, Error> {
    let ident = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new(field.span(), "Only named fields are supported"))?;
    let ident_str = ident.to_string();
    let help = attr
        .help
        .or_else(|| parse_doc_first_line(&field.attrs))
        .unwrap_or_else(|| ident_str.clone());
    let is_family = attr.family || is_family_type(&field.ty);
    Ok(FieldInfo {
        ident,
        ident_str,
        help,
        is_family,
    })
}

/// Checks if a type is `Family<_, _>`.
fn is_family_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Family" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Family should have 2 generic arguments
                    return args
                        .args
                        .iter()
                        .filter(|arg| matches!(arg, GenericArgument::Type(_)))
                        .count()
                        == 2;
                }
            }
        }
    }
    false
}

fn expand_metrics(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let (name, fields) = parse_named_struct(input)?;
    let attr = parse_metrics_attr(&input.attrs)?;
    let name_str = attr
        .name
        .unwrap_or_else(|| name.to_string().to_snake_case());

    let default = if attr.default {
        let mut items = vec![];
        for field in fields.iter() {
            let ident = field
                .ident
                .as_ref()
                .ok_or_else(|| Error::new(field.span(), "Only named fields are supported"))?;
            let attr = parse_default_attr(&field.attrs)?;
            let item = if let Some(expr) = attr {
                quote!( #ident: #expr)
            } else {
                quote!( #ident: ::std::default::Default::default() )
            };
            items.push(item);
        }
        Some(quote! {
            impl ::std::default::Default for #name {
                fn default() -> Self {
                    Self {
                        #(#items),*
                    }
                }
            }
        })
    } else {
        None
    };

    Ok(quote! {
        impl ::iroh_metrics::MetricsGroup for #name {
            fn name(&self) -> &'static str {
                #name_str
            }
        }

        #default
    })
}

fn expand_metrics_group_set(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let (name, fields) = parse_named_struct(input)?;
    let attr = parse_metrics_attr(&input.attrs)?;
    let name_str = attr
        .name
        .unwrap_or_else(|| name.to_string().to_snake_case());

    let mut cloned = quote! {};
    let mut refs = quote! {};
    for field in fields {
        let name = field.ident.as_ref().unwrap();
        cloned.extend(quote! {
            self.#name.clone() as ::std::sync::Arc<dyn ::iroh_metrics::MetricsGroup>,
        });
        refs.extend(quote! {
            &*self.#name as &dyn ::iroh_metrics::MetricsGroup,
        });
    }

    Ok(quote! {
        impl ::iroh_metrics::MetricsGroupSet for #name {
            fn name(&self) -> &'static str {
                #name_str
            }

            fn groups_cloned(&self) -> impl ::std::iter::Iterator<Item = ::std::sync::Arc<dyn ::iroh_metrics::MetricsGroup>> {
                [#cloned].into_iter()
            }

            fn groups(&self) -> impl ::std::iter::Iterator<Item = &dyn ::iroh_metrics::MetricsGroup> {
                [#refs].into_iter()
            }
        }
    })
}

fn expand_encode_label_set(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let (name, fields) = parse_named_struct(input)?;
    let struct_attr = parse_label_struct_attr(&input.attrs)?;

    let mut label_pairs = vec![];
    for field in fields.iter() {
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| Error::new(field.span(), "Only named fields are supported"))?;
        let attr = parse_label_attr(&field.attrs)?;

        // Skip fields marked with #[label(skip)]
        if attr.skip {
            continue;
        }

        // Field-level `name = ...` wins; otherwise apply the struct-level
        // `rename_all` transformation to the field ident.
        let label_name = match attr.name {
            Some(n) => n,
            None => match struct_attr.rename_all {
                Some(rule) => rule.apply(&ident.to_string()),
                None => ident.to_string(),
            },
        };

        // Borrow the field through `EncodeLabelValue` so string fields
        // don't allocate on every scrape.
        label_pairs.push(quote! {
            (#label_name, ::iroh_metrics::EncodeLabelValue::encode_label_value(&self.#ident))
        });
    }

    Ok(quote! {
        impl ::iroh_metrics::EncodeLabelSet for #name {
            fn encode_label_pairs(&self) -> ::std::vec::Vec<::iroh_metrics::LabelPair<'_>> {
                ::std::vec![#(#label_pairs),*]
            }
        }
    })
}

fn expand_encode_label_value(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(Error::new(
            input.span(),
            "EncodeLabelValue can only be derived for enums with unit variants.",
        ));
    };
    let enum_attr = parse_label_struct_attr(&input.attrs)?;
    let rule = enum_attr.rename_all.unwrap_or(RenameRule::SnakeCase);

    let mut arms = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new(
                variant.span(),
                "EncodeLabelValue only supports unit variants.",
            ));
        }
        let attr = parse_label_attr(&variant.attrs)?;
        let ident = &variant.ident;
        let label = attr.name.unwrap_or_else(|| rule.apply(&ident.to_string()));
        arms.push(quote! {
            Self::#ident => ::iroh_metrics::LabelValue::Str(::std::borrow::Cow::Borrowed(#label)),
        });
    }

    Ok(quote! {
        impl ::iroh_metrics::EncodeLabelValue for #name {
            fn encode_label_value(&self) -> ::iroh_metrics::LabelValue<'_> {
                match self {
                    #(#arms)*
                }
            }
        }
    })
}

#[derive(Clone, Copy)]
enum RenameRule {
    SnakeCase,
    CamelCase,
    PascalCase,
    ScreamingSnakeCase,
    KebabCase,
    Lowercase,
    Uppercase,
}

impl RenameRule {
    fn parse(s: &str, span: proc_macro2::Span) -> Result<Self, Error> {
        match s {
            "snake_case" => Ok(Self::SnakeCase),
            "camelCase" => Ok(Self::CamelCase),
            "PascalCase" => Ok(Self::PascalCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "lowercase" => Ok(Self::Lowercase),
            "UPPERCASE" => Ok(Self::Uppercase),
            other => Err(Error::new(
                span,
                format!(
                    "unknown rename_all value `{other}`. Supported: snake_case, camelCase, \
                     PascalCase, SCREAMING_SNAKE_CASE, kebab-case, lowercase, UPPERCASE.",
                ),
            )),
        }
    }

    fn apply(self, ident: &str) -> String {
        match self {
            Self::SnakeCase => ident.to_snake_case(),
            Self::CamelCase => ident.to_lower_camel_case(),
            Self::PascalCase => ident.to_pascal_case(),
            Self::ScreamingSnakeCase => ident.to_shouty_snake_case(),
            Self::KebabCase => ident.to_kebab_case(),
            Self::Lowercase => ident.to_lowercase(),
            Self::Uppercase => ident.to_uppercase(),
        }
    }
}

#[derive(Default)]
struct LabelStructAttr {
    rename_all: Option<RenameRule>,
}

fn parse_label_struct_attr(attrs: &[Attribute]) -> Result<LabelStructAttr, Error> {
    let mut out = LabelStructAttr::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("label")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let s: LitStr = meta.value()?.parse()?;
                out.rename_all = Some(RenameRule::parse(&s.value(), s.span())?);
                Ok(())
            } else {
                Err(meta.error("The struct-level `label` attribute supports only `rename_all`."))
            }
        })?;
    }
    Ok(out)
}

fn parse_doc_first_line(attrs: &[Attribute]) -> Option<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .flat_map(|attr| attr.meta.require_name_value())
        .find_map(|name_value| {
            let Expr::Lit(ExprLit { lit, .. }) = &name_value.value else {
                return None;
            };
            let Lit::Str(str) = lit else { return None };
            Some(str.value().trim().to_string())
        })
}

#[derive(Default)]
struct MetricsAttr {
    name: Option<String>,
    help: Option<String>,
    default: bool,
    /// `#[metrics(family)]` — force-treat the field as a `Family<_, _>`
    /// even when the type is hidden behind an alias.
    family: bool,
}

#[derive(Default)]
struct LabelAttr {
    name: Option<String>,
    skip: bool,
}

fn parse_default_attr(attrs: &[Attribute]) -> Result<Option<syn::Expr>, syn::Error> {
    let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident("default")) else {
        return Ok(None);
    };
    let expr = attr.parse_args::<Expr>()?;
    Ok(Some(expr))
}

fn parse_metrics_attr(attrs: &[Attribute]) -> Result<MetricsAttr, syn::Error> {
    let mut out = MetricsAttr::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("metrics")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                out.name = Some(parse_lit_str(&meta)?);
                Ok(())
            } else if meta.path.is_ident("help") {
                out.help = Some(parse_lit_str(&meta)?);
                Ok(())
            } else if meta.path.is_ident("default") {
                out.default = true;
                Ok(())
            } else if meta.path.is_ident("family") {
                out.family = true;
                Ok(())
            } else {
                Err(meta.error(
                    "The `metrics` attribute supports only `name`, `help`, `default`, and `family`.",
                ))
            }
        })?;
    }
    Ok(out)
}

fn parse_label_attr(attrs: &[Attribute]) -> Result<LabelAttr, syn::Error> {
    let mut out = LabelAttr::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("label")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                out.name = Some(parse_lit_str(&meta)?);
                Ok(())
            } else if meta.path.is_ident("skip") {
                out.skip = true;
                Ok(())
            } else {
                Err(meta.error("The `label` attribute supports only `name` and `skip` fields."))
            }
        })?;
    }
    Ok(out)
}

fn parse_lit_str(meta: &ParseNestedMeta<'_>) -> Result<String, Error> {
    let s: LitStr = meta.value()?.parse()?;
    Ok(s.value().trim().to_string())
}

fn parse_named_struct(input: &DeriveInput) -> Result<(&Ident, &Fields), Error> {
    match &input.data {
        Data::Struct(data) if matches!(data.fields, Fields::Named(_)) => {
            Ok((&input.ident, &data.fields))
        }
        _ => Err(Error::new(
            input.span(),
            "The `MetricsGroup` and `Iterable` derives support only structs.",
        )),
    }
}
