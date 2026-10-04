macro_rules! ast_struct {
    (
        [$($attrs_pub:tt)*]
        struct $name:ident $($rest:tt)*
    ) => {
        #[cfg_attr(feature = "clone-impls", derive(Clone))]
        $($attrs_pub)* struct $name $($rest)*
    };

    ($($tt:tt)*) => {
        strip_attrs_pub!(ast_struct!($($tt)*));
    };
}

macro_rules! ast_enum {
    (
        [$($attrs_pub:tt)*]
        enum $name:ident $($rest:tt)*
    ) => (
        #[cfg_attr(feature = "clone-impls", derive(Clone))]
        $($attrs_pub)* enum $name $($rest)*
    );

    ($($tt:tt)*) => {
        strip_attrs_pub!(ast_enum!($($tt)*));
    };
}

macro_rules! ast_enum_of_structs {
    (
        $(#[$enum_attr:meta])*
        $pub:ident $enum:ident $name:ident $body:tt
    ) => {
        ast_enum!($(#[$enum_attr])* $pub $enum $name $body);
        ast_enum_of_structs_impl!($pub $enum $name $body);
    };
}

macro_rules! ast_enum_of_structs_impl {
    (
        $pub:ident $enum:ident $name:ident {
            $(
                $(#[$variant_attr:meta])*
                $variant:ident $( ($member:ident) )*,
            )*
        }
    ) => {
        check_keyword_matches!(pub $pub);
        check_keyword_matches!(enum $enum);

        $(
            $(
                impl From<$member> for $name {
                    fn from(e: $member) -> $name {
                        $name::$variant(e)
                    }
                }
            )*
        )*

        generate_to_tokens! {
            ()
            tokens
            $name { $($variant $($member)*,)* }
        }
    };
}

macro_rules! generate_to_tokens {
    (($($arms:tt)*) $tokens:ident $name:ident { $variant:ident, $($next:tt)*}) => {
        generate_to_tokens!(
            ($($arms)* $name::$variant => {})
            $tokens $name { $($next)* }
        );
    };

    (($($arms:tt)*) $tokens:ident $name:ident { $variant:ident $member:ident, $($next:tt)*}) => {
        generate_to_tokens!(
            ($($arms)* $name::$variant(_e) => quote::ToTokens::to_tokens(_e, $tokens),)
            $tokens $name { $($next)* }
        );
    };

    (($($arms:tt)*) $tokens:ident $name:ident {}) => {
        impl quote::ToTokens for $name {
            fn to_tokens(&self, $tokens: &mut proc_macro2::TokenStream) {
                match self {
                    $($arms)*
                }
            }
        }
    };
}

macro_rules! strip_attrs_pub {
    ($mac:ident!($(#[$m:meta])* $pub:ident $($tt:tt)*)) => {
        check_keyword_matches!(pub $pub);

        $mac!([$(#[$m])* $pub] $($tt)*);
    };
}

macro_rules! check_keyword_matches {
    (enum enum) => {};
    (pub pub) => {};
}
