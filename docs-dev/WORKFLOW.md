# Workflow

State machine helpers live in **`serenade-workflow`** ([#239](https://github.com/Interchouette-ITC/Serenade/issues/239)).

Symfony Workflow shaped: places, transitions, guards, marking store. No commerce types.

## Concepts

| Piece | Role |
| --- | --- |
| `Definition` / `DefinitionBuilder` | Places, transitions, initial marking |
| `Transition` | Named edge `from` → `to` (one or many places each side) |
| `Marking` | Places currently occupied by a subject |
| `MarkingStore` / `MemoryMarkingStore` | Persist markings by subject id |
| `Workflow` | `can` / `apply` / `enabled_transitions` |
| `Guard` | Block a transition before it applies (Rust closure or [`ExpressionGuard`](EXPRESSION.md)) |
| `ExpressionGuard` | Guard from a boolean expression string (see [EXPRESSION.md](EXPRESSION.md)) |
| `TransitionListener` | Hook after a successful `apply` |

## Example

```rust
use std::sync::Arc;
use serenade_workflow::{DefinitionBuilder, MemoryMarkingStore, Workflow};

let definition = DefinitionBuilder::new()
    .places(["draft", "published", "archived"])
    .edge("publish", "draft", "published")
    .edge("archive", "published", "archived")
    .build()?;
let store = Arc::new(MemoryMarkingStore::new());
let workflow = Workflow::new("article", definition, store);

assert!(workflow.can("post-1", "publish"));
workflow.apply("post-1", "publish")?;
assert!(!workflow.can("post-1", "publish"));
assert!(workflow.can("post-1", "archive"));
```

## Guards

Register per transition name, or `"*"` for every transition:

```rust
use std::sync::Arc;
use serenade_workflow::{Guard, TransitionContext, Workflow, block};

workflow.add_guard(
    "publish",
    Arc::new(|ctx: &TransitionContext<'_>| {
        if ctx.subject_id.starts_with("guest:") {
            Err(block(ctx.transition.name(), "guests cannot publish"))
        } else {
            Ok(())
        }
    }),
);
```

Or use a rule expression ([EXPRESSION.md](EXPRESSION.md)):

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

## Limits

- Subject identity is a string id (apps map entities → ids)
- In-memory store only in this crate; implement [`MarkingStore`] for DB/Redis
- No DI compile pass yet (construct `Workflow` in app code)

## Related

- [EXPRESSION.md](EXPRESSION.md) - safe rule formulas / `ExpressionGuard`
- [KERNEL.md](KERNEL.md) - component index
- [STRING.md](STRING.md) - slug / case helpers often used near workflow place names
