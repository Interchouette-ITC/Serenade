# serenade-workflow

State machine places, transitions, guards, and marking store (Symfony Workflow
shaped).

`DefinitionBuilder` declares places, transitions, and the initial marking;
`Workflow` answers `can` / `apply` / `enabled_transitions`; `MarkingStore`
(`MemoryMarkingStore`) persists subject markings; `Guard` / `ExpressionGuard` block
illegal or policy-denied transitions.

Part of [Serenade](https://github.com/Interchouette-ITC/Serenade); most applications
depend on the [`serenade-framework`](https://crates.io/crates/serenade-framework) facade.
API documentation: <https://docs.rs/serenade-workflow>.

Licensed under Apache-2.0.
