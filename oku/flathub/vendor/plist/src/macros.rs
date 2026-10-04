/// Constructs a `plist::Value` from a JSON-like literal.
///
/// ```
/// # use plist::plist;
/// #
/// let value = plist!({
///     "code": 200,
///     "success": true,
///     "payload": {
///         "features": [
///             "serde",
///         ]
///     }
/// });
/// ```
///
/// Variables or expressions can be interpolated into the literal. Any type
/// interpolated into an array element or object value must implement the
/// `Into<Value>` trait, while any type interpolated into a object key must
/// implement `Into<String>`.
///
/// ```
/// # use plist::plist;
/// #
/// let code = 200;
/// let features = vec!["serde", "plist"];
///
/// let value = plist!({
///     "code": code,
///     "success": code == 200,
///     "payload": {
///         features[0]: features[1]
///     }
/// });
/// ```
///
/// Trailing commas are allowed inside both arrays and objects.
///
/// ```
/// # use plist::plist;
/// #
/// let value = plist!([
///     "notice",
///     "the",
///     "trailing",
///     "comma -->",
/// ]);
/// ```
#[macro_export(local_inner_macros)]
macro_rules! plist {
    // Hide distracting implementation details from the generated rustdoc.
    ($($plist:tt)+) => {
        plist_internal!($($plist)+)
    };
}

/// Constructs a `plist::Dictionary` from a JSON-like literal.
///
/// ```
/// # use plist::plist_dict;
/// #
/// let value = plist_dict! {
///     "code": 200,
///     "success": true,
///     "payload": {
///         "features": [
///             "serde",
///         ]
///     }
/// };
/// ```
///
/// Variables or expressions can be interpolated into the literal. Any type
/// interpolated into an array element or object value must implement the
/// `Into<Value>` trait, while any type interpolated into a object key must
/// implement `Into<String>`.
///
/// ```
/// # use plist::plist_dict;
/// #
/// let code = 200;
/// let features = vec!["serde", "plist"];
///
/// let value = plist_dict! {
///     "code": code,
///     "success": code == 200,
///     "payload": {
///         features[0]: features[1]
///     }
/// };
/// ```
///
/// Trailing commas are allowed inside both arrays and objects.
///
/// ```
/// # use plist::plist_dict;
/// #
/// let value = plist_dict! {
///     "notice": 0,
///     "the": 1,
///     "trailing": 2,
///     "comma -->": 3,
/// };
/// ```
#[macro_export(local_inner_macros)]
macro_rules! plist_dict {
    () => {
        $crate::Dictionary::new()
    };
    // Allow outer curlies if provided
    ({$($tt:tt)+}) => {
        {
            let mut object = $crate::Dictionary::new();
            plist_internal!(@object object () ($($tt)+) ($($tt)+));
            object
        }
    };
    ($($tt:tt)+) => {
        {
            let mut object = $crate::Dictionary::new();
            plist_internal!(@object object () ($($tt)+) ($($tt)+));
            object
        }
    };
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! plist_internal {
    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an array [...]. Produces a vec![...]
    // of the elements.
    //
    // Must be invoked as: plist_internal!(@array [] $($tt)*)
    //////////////////////////////////////////////////////////////////////////

    // Done with trailing comma.
    (@array [$($elems:expr,)*]) => {
        plist_internal_vec![$($elems,)*]
    };

    // Done without trailing comma.
    (@array [$($elems:expr),*]) => {
        plist_internal_vec![$($elems),*]
    };

    // Next element is `true`.
    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        plist_internal!(@array [$($elems,)* plist_internal!(true)] $($rest)*)
    };

    // Next element is `false`.
    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        plist_internal!(@array [$($elems,)* plist_internal!(false)] $($rest)*)
    };

    // Next element is an array.
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        plist_internal!(@array [$($elems,)* plist_internal!([$($array)*])] $($rest)*)
    };

    // Next element is a map.
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        plist_internal!(@array [$($elems,)* plist_internal!({$($map)*})] $($rest)*)
    };

    // Next element is an expression followed by comma.
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        plist_internal!(@array [$($elems,)* plist_internal!($next),] $($rest)*)
    };

    // Last element is an expression with no trailing comma.
    (@array [$($elems:expr,)*] $last:expr) => {
        plist_internal!(@array [$($elems,)* plist_internal!($last)])
    };

    // Comma after the most recent element.
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        plist_internal!(@array [$($elems,)*] $($rest)*)
    };

    // Unexpected token after most recent element.
    (@array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        plist_unexpected!($unexpected)
    };

    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an object {...}. Each entry is
    // inserted into the given map variable.
    //
    // Must be invoked as: plist_internal!(@object $map () ($($tt)*) ($($tt)*))
    //
    // We require two copies of the input tokens so that we can match on one
    // copy and trigger errors on the other copy.
    //////////////////////////////////////////////////////////////////////////

    // Done.
    (@object $object:ident () () ()) => {};

    // Insert the current entry followed by trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(($($key)+).into(), $value);
        plist_internal!(@object $object () ($($rest)*) ($($rest)*));
    };

    // Current entry followed by unexpected token.
    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        plist_unexpected!($unexpected);
    };

    // Insert the last entry without trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.insert(($($key)+).into(), $value);
    };

    // Next value is `true`.
    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        plist_internal!(@object $object [$($key)+] (plist_internal!(true)) $($rest)*);
    };

    // Next value is `false`.
    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        plist_internal!(@object $object [$($key)+] (plist_internal!(false)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        plist_internal!(@object $object [$($key)+] (plist_internal!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        plist_internal!(@object $object [$($key)+] (plist_internal!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        plist_internal!(@object $object [$($key)+] (plist_internal!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        plist_internal!(@object $object [$($key)+] (plist_internal!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        plist_internal!();
    };

    // Missing colon and value for last entry. Trigger a reasonable error
    // message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        plist_internal!();
    };

    // Misplaced colon. Trigger a reasonable error message.
    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        plist_unexpected!($colon);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        plist_unexpected!($comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        plist_internal!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Refuse to absorb colon token into key expression.
    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        plist_expect_expr_comma!($($unexpected)+);
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        plist_internal!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    //////////////////////////////////////////////////////////////////////////
    // The main implementation.
    //
    // Must be invoked as: plist_internal!($($plist)+)
    //////////////////////////////////////////////////////////////////////////

    (true) => {
        $crate::Value::Boolean(true)
    };

    (false) => {
        $crate::Value::Boolean(false)
    };

    ([]) => {
        $crate::Value::Array(plist_internal_vec![])
    };

    ([ $($tt:tt)+ ]) => {
        $crate::Value::Array(plist_internal!(@array [] $($tt)+))
    };

    ({}) => {
        $crate::Value::Dictionary($crate::Dictionary::new())
    };

    ({ $($tt:tt)+ }) => {
        $crate::Value::Dictionary({
            let mut object = $crate::Dictionary::new();
            plist_internal!(@object object () ($($tt)+) ($($tt)+));
            object
        })
    };

    // Any Into<plist::Value> type: numbers, strings, struct literals, variables etc.
    // Must be below every other rule.
    ($other:expr) => {
        $crate::Value::from($other)
    };
}

// The plist_internal macro above cannot invoke vec directly because it uses
// local_inner_macros. A vec invocation there would resolve to plist::vec.
// Instead invoke vec here outside of local_inner_macros.
#[macro_export]
#[doc(hidden)]
macro_rules! plist_internal_vec {
    ($($content:tt)*) => {
        vec![$($content)*]
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! plist_unexpected {
    () => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! plist_expect_expr_comma {
    ($e:expr , $($tt:tt)*) => {};
}

#[cfg(test)]
mod tests {
    use crate::Dictionary;

    #[test]
    fn plist_dict() {
        let expected = Dictionary::from_iter([(String::from("foo"), String::from("bar"))]);

        let d = plist_dict! {
            "foo": "bar",
        };
        assert_eq!(&d, &expected);

        let d = plist_dict!({
            "foo": "bar",
        });
        assert_eq!(&d, &expected);
    }
}
