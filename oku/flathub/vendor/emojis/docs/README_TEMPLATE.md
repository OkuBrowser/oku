# {{ manifest.name }}

{% if config.badges.crates_io -%}
[![Crates.io Version](https://badgers.space/crates/version/{{ manifest.name }})](https://crates.io/crates/{{ manifest.name }})
{% endif -%}

{% if config.badges.docs_rs -%}
[![Docs.rs Latest](https://badgers.space/badge/docs.rs/latest/blue)](https://docs.rs/{{ manifest.name }})
{% endif -%}

{% if config.badges.github_workflow -%}
[![Build Status](https://badgers.space/github/checks/{{ manifest.repository | trim_prefix: "https://github.com/" }}?label={{ config.badges.github_workflow.label }})]({{ manifest.repository }}/actions/workflows/{{ config.badges.github_workflow.name }}.yaml)
{%- endif %}

{{ summary }}

{{ contents }}

## License

This project is licensed under `(MIT OR Apache-2.0) AND Unicode-3.0`.

All intellectual property within this crate that is not generated from Unicode
data is dual-licensed under either the MIT License or the Apache License,
Version 2.0, at your option.

This crate makes use of data derived from the Unicode Character Database
and related Unicode sources. All generated files and data derived from
Unicode, including but not limited to emoji names, ordering, group
classifications, skin tone metadata, Unicode version metadata, and
lookup tables in `src/gen/`, are additionally governed by the Unicode
License, Version 3.0.

When using or redistributing generated files that incorporate Unicode
data, you must comply with the terms of both the Unicode License and
either the MIT or Apache-2.0 license.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project shall be dual-licensed under
the same terms, without additional conditions.

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[LICENSE-UNICODE](LICENSE-UNICODE) for full license texts.
