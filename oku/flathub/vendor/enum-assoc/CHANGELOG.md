# 1.4.0

- Updated to Rust 2024

# 1.3.0

- Implemented the ability to put multiple functions in a single func attribute, allowing their "assoc" values to be inferred.

# 1.2.4

- Better tests.

# 1.2.2

- More parser improvements
- Minor cleanup

# 1.2.0

- Implemented comma-separated associations in a single attribute
- Parsing logic improved to be more canon and sustainable (but still could be more so)

# 1.1.0

- Now capable of referencing variant fields within assoc.

# 1.0.0

- No significant changes. Bugs have not been reported since before 0.4.0 so this is going 1.0.0.

# 0.4.0
- `func` attributes now now specify default return values.

# 0.3.4
- Added limited support for lifetimes and generics. Trait bounds still not implemented properly.

# 0.3.1
- Now with guaranteed ordering 
- Added more testing 
- Better docs 

# 0.3.0
- Added more errors when something unreasonable is done

# 0.2.0
- Added reverse associations.
- Code quality improvement, using `syn` more and string manipulation less.

# 0.1.8
- Fixed a bug which would preclude the use of generics in function signature, provided better documentation.

# 0.1.5
- Implemented more useful error handling using `syn::Error`.

# 0.1.4
- Initial release, some comment and documentation updates.