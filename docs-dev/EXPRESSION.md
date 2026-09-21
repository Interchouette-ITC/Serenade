# Expression

Tiny safe rule formulas live in **`serenade-expression`** ([#251](https://github.com/Interchouette-ITC/Serenade/issues/251)).

Symfony ExpressionLanguage shaped: boolean / arithmetic / dotted property paths only.
Not a scripting language. No function calls. No arbitrary Rust eval.

## Pieces

| Type | Role |
| --- | --- |
| `Value` | `Bool`, `Int`, `Str` |
| `ExpressionContext` | Flat key → value map (`user.role` is one key) |
| `evaluate` / `evaluate_bool` | Parse + run |
| `ExpressionError` | Parse, unknown variable, type, division by zero |
| `ExpressionGuard` (in `serenade-workflow`) | Workflow guard from an expression string |

## Operators

`==` `!=` `<` `<=` `>` `>=` `&&` `||` `!` `+` `-` `*` `/` and parentheses.
Literals: `true` / `false`, integers, `"strings"` (escapes `\"` `\\` `\n` `\t`).

## Example (security-style check)

```rust
use serenade_expression::{ExpressionContext, Value, evaluate_bool};

let mut ctx = ExpressionContext::new();
ctx.insert("user.role", Value::string("admin"));
ctx.insert("count", Value::Int(2));
assert!(evaluate_bool(r#"user.role == "admin" && count > 0"#, &ctx)?);
```

## Example (workflow guard)

```rust
use std::sync::Arc;
use serenade_workflow::ExpressionGuard;

workflow.add_guard(
    "publish",
    Arc::new(ExpressionGuard::new(
        r#"!(subject_id == "guest") && place.draft"#,
    )),
);
```

Built-in guard variables: `subject_id`, `workflow`, `transition`, and `place.<name>` for each
place in the current marking.

## Related

- [WORKFLOW.md](WORKFLOW.md) - `ExpressionGuard`
- [SECURITY.md](SECURITY.md) - using expressions for access checks
- [KERNEL.md](KERNEL.md) - component index
