# enum-assoc

This crate defines a few macros that allow you to associate constants or data with enum variants.

To use, `#[derive(Assoc)]` must be attached to an enum. From there, the `func` attribute is used to define function signatures which will be implemented for that enum. The `assoc` attribute is used to define constants which each variant will return when that function is called.

## Forward associations

Here's an example:

```rust
use enum_assoc::Assoc;

const WA: &'static str = "wa";

#[derive(Assoc)]
#[func(pub const fn foo(&self) -> u8)]
#[func(pub fn bar(&self) -> &'static str)]
#[func(pub fn maybe_foo(&self) -> Option<u8>)]
#[func(pub fn with_default(&self) -> u8 { 4 })]
enum TestEnum {
    #[assoc(foo = 255, bar = "wow")]
    Variant1,
    #[assoc(foo = 1 + 7, bar = "wee", with_default = 2)]
    Variant2,
    #[assoc(foo = 0, bar = WA, maybe_foo = 18 + 2)]
    Variant3
}

fn main() {
    println!("Variant1 foo: {}", TestEnum::Variant1.foo());
    println!("Variant2 foo: {}", TestEnum::Variant2.foo());
    println!("Variant3 foo: {}", TestEnum::Variant3.foo());
    println!("Variant1 bar: {}", TestEnum::Variant1.bar());
    println!("Variant2 bar: {}", TestEnum::Variant2.bar());
    println!("Variant3 bar: {}", TestEnum::Variant3.bar());
    println!("Variant1 maybe_foo: {:?}", TestEnum::Variant1.maybe_foo());
    println!("Variant2 maybe_foo: {:?}", TestEnum::Variant2.maybe_foo());
    println!("Variant3 maybe_foo: {:?}", TestEnum::Variant3.maybe_foo());
    println!("Variant1 with_default: {:?}", TestEnum::Variant1.with_default());
    println!("Variant2 with_default: {:?}", TestEnum::Variant2.with_default());
    println!("Variant3 with_default: {:?}", TestEnum::Variant3.with_default());
}

```

Output:

```ignore
Variant1 foo: 255
Variant2 foo: 8
Variant3 foo: 0
Variant1 bar: wow
Variant2 bar: wee
Variant3 bar: wa
Variant1 maybe_foo: None
Variant2 maybe_foo: None
Variant3 maybe_foo: Some(20)
Variant1 with_default: 4
Variant2 with_default: 2
Variant3 with_default: 4
```

Note that functions which return an `Option` type have special functionality: Variants may leave out the `assoc` attribute entirely to automatically return `None`, and variants which do yield a value need not explicitly wrap it in `Some`.

### What does this output?

Every `#[func(fn_signature)]` attribute generates something like the following:

```rust,ignore
impl Enum {
    fn_signature {
        match self {
            // ... arms
        }
    }
}
```

And every `#[assoc(fn_name = association)]` attribute generates an arm for its associated function like the following:

```rust,ignore
    variant_name => association,
```

That's it. Both the details of the `fn_signature` you use and what you put in the `association` area are up to you.

So while technically not the original intention of this crate, you can generate some more interesting/complex associations for free:

```rust
use enum_assoc::Assoc;

#[derive(Assoc)]
#[func(pub fn foo(&self, param: u8) -> Option<u8>)]
#[func(pub fn bar(&self, param: &str) -> String)]
#[func(pub fn baz<T: std::fmt::Debug>(&self, param: T) -> Option<String>)]
enum TestEnum2 {
    #[assoc(bar = String::new() + param)]
    Variant1,
    #[assoc(foo = 16 + param)]
    #[assoc(bar = String::from("Hello") + param)]
    Variant2,
    #[assoc(bar = some_str_func(param))]
    #[assoc(baz = format!("{:?}", param))]
    Variant3
}

fn some_str_func(s: &str) -> String {
    String::from("I was created in a function") + s
}

fn main() {
    println!("Variant1 foo: {:?}", TestEnum2::Variant1.foo(0));
    println!("Variant2 foo: {:?}", TestEnum2::Variant2.foo(22));
    println!("Variant1 bar: {}", TestEnum2::Variant1.bar("string"));
    println!("Variant2 bar: {}", TestEnum2::Variant2.bar(" World!"));
    println!("Variant3 bar: {}", TestEnum2::Variant3.bar("!"));
    println!("Variant3 baz: {:?}", TestEnum2::Variant3.baz(1));
}
```

Output:

```ignore
Variant1 foo: None
Variant2 foo: 34
Variant1 bar: string
Variant2 bar: Hello World!
Variant3 bar: I was created in a function!
Variant3 baz: Some("1")
```

## Accessing enum fields in `assoc` attribute

It is possible to access an enum variant field value in an assoc attribute
by prefixing its name with an underscore. For tuples the name is composed of
an underscore prefix and the field index.

```rust,ignore

#[func(pub fn foo(&self) -> usize]
pub enum TestEnum1 {
    #[assoc(foo = _s.len())]
    Variant1 { s: String },
}

#[func(pub fn foo(&self) -> usize)]
pub enum TestEnum1 {
    #[assoc(foo = _0.len())]
    Variant1(String),
}

```

## Reverse associations

This can also generate reverse associations (constants to enum variants). See below for an example.

```rust
use enum_assoc::Assoc;

#[derive(Assoc, Debug)]
#[func(pub fn foo(s: &str) -> Option<Self>)]
#[func(pub fn bar(u: u8) -> Self)]
#[func(pub fn baz(u1: u8, u2: u8) -> Self)]
enum TestEnum3
{
    #[assoc(foo = "variant1")]
    #[assoc(bar = _)]
    Variant1,
    #[assoc(bar = 2)]
    #[assoc(foo = "variant2")]
    #[assoc(baz = (3, 7))]
    Variant2,
    #[assoc(foo = "I'm variant 3!")]
    #[assoc(foo = "variant3")]
    #[assoc(baz = _)]
    Variant3
}

fn main()
{
    println!("TestEnum3 foo(\"variant1\"): {:?}", TestEnum3::foo("variant1"));
    println!("TestEnum3 foo(\"variant3\"): {:?}", TestEnum3::foo("variant3"));
    println!("TestEnum3 foo(\"I'm variant 3!\"): {:?}", TestEnum3::foo("I'm variant 3!"));
    println!("TestEnum3 foo(\"I don't exist\"): {:?}", TestEnum3::foo("I don't exist"));
    println!("TestEnum3 bar(2): {:?}", TestEnum3::bar(2));
    println!("TestEnum3 bar(55): {:?}", TestEnum3::bar(55));
    println!("TestEnum3 baz(3, 7): {:?}", TestEnum3::baz(3, 7));
    println!("TestEnum3 baz(0, 0): {:?}", TestEnum3::baz(0, 0));
}
```

Output:

```ignore
TestEnum3 foo("variant1"): Some(Variant1)
TestEnum3 foo("variant3"): Some(Variant3)
TestEnum3 foo("I'm variant 3!"): Some(Variant3)
TestEnum3 foo("I don't exist"): None
TestEnum3 bar(2): Variant2
TestEnum3 bar(55): Variant1
TestEnum3 baz(3, 7): Variant2
TestEnum3 baz(0, 0): Variant3
```

Reverse associations work slightly differently than forward associations:

- Reverse associations must not include a `self` parameter (the lack of a `self` paramater is what differentiates a forward association from a reverse association)
- They must return either `Self` or `Option<Self>`
- Unlike forward associations, any number of `assoc` attributes for the same function may be defined for a single enum variant.
- Unlike forward associations, the `assoc` attribute defines a pattern rather than an expression. This is because reverse associations control the left side of a match arm rather than the right side.
- The function generated will match on a tuple containing all of the function arguments.
- Match arms will be ordered exactly as written from top to bottom with one exception: any wildcard pattern `_` will always be placed at the bottom.
- There can be no more than 1 wildcard association for any reverse-associative function. Any more will result in a compile error.
- If no wildcard pattern is defined for a function that returns `Option<Self>`, a `_ => None` arm will be inserted automatically.

So for a simple reverse association to generate valid code, 1 of these 3 conditions must be satisfied:

1. The reverse association returns `Option<Self>`, or
2. A wildcard (`_`) pattern is defined for exactly 1 variant, or
3. Every possible value maps to an enum variant

- Note: For reverse associations that take more than 1 argument, it is possible to use wildcards for specific arguments (eg `(5, _)`). This macro does not attempt to re-order this in the same way it does to catch-all wildcards (`_`). The match arm will be placed exactly where it appears in the column of enum attributes.

Currently, there is no way for reverse associations to map to tuple or struct-like variants.

### What does this output?

Every `#[func(fn_signature)]` attribute for reverse associations generates something like the following:

```rust,ignore
impl Enum {
    fn_signature {
        match (param1, param2, etc) {
            // ... arms
        }
    }
}
```

And every `#[assoc(fn_name = pattern)]` attribute for reverse associations generates an arm for its associated function like the following:

```rust,ignore
    pattern => variant_name,
```

## Additional Syntax Sugar

`func` attributes may contain multiple functions, forward or reverse. This will allow associations of _any_ of those functions to apply to all of them:

```rust,ignore
#[derive(Assoc, PartialEq, Debug)]
#[func(
    pub fn forward(&self) -> u8, 
    pub fn reverse(rev: u8) -> Option<Self>
)]
enum TestMultiFunc {
    #[assoc(forward = 10)]
    Variant,
}
assert_eq!(TestMultiFunc::Variant.forward(), 10);
assert_eq!(TestMultiFunc::reverse(10), Some(TestMultiFunc::Variant));
```

Note that because the value in the assoc attribute must be correctly parsable as both an expression and a pattern, this creates additional expressability limitations.
