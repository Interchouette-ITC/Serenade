# String / Inflector

Slug, case transforms, and English Inflector helpers live in **`serenade-string`** ([#229](https://github.com/Interchouette-ITC/Serenade/issues/229)).

Zero dependencies. Not a substitute for i18n plurals ([I18N.md](I18N.md)).

## API

| Function | Example |
| --- | --- |
| `slug` | `"Hello World!"` → `"hello-world"` |
| `snake_case` | `"HelloWorld"` → `"hello_world"` |
| `kebab_case` | `"HelloWorld"` → `"hello-world"` |
| `camel_case` | `"hello_world"` → `"helloWorld"` |
| `pascal_case` | `"hello_world"` → `"HelloWorld"` |
| `title_case` | `"hello_world"` → `"Hello World"` |
| `pluralize` | `"baby"` → `"babies"`, `"person"` → `"people"` |
| `singularize` | `"babies"` → `"baby"`, `"people"` → `"person"` |

## Limits (Inflector)

- English only
- Small irregular table (`person/people`, `child/children`, …) and uncountables (`sheep`, `news`, …)
- Common suffix rules (`y` → `ies`, `f/fe` → `ves`, `s/x/z/ch/sh/o` → `es`)
- Not a full linguistic Inflector; apps with domain-specific nouns should keep their own map

## Example

```rust
use serenade_string::{camel_case, pluralize, slug, snake_case};

assert_eq!(slug("Rusta Shop"), "rusta-shop");
assert_eq!(snake_case("OrderItem"), "order_item");
assert_eq!(camel_case("order_item"), "orderItem");
assert_eq!(pluralize("category"), "categories");
```

## Related

- [I18N.md](I18N.md) - message catalogues and ICU plurals for UI copy
- [KERNEL.md](KERNEL.md) - component index
