# Persistence adapters

Serenade defines **generic persistence contracts**; applications own schema, migrations, ORM/SQL choices, and **domain repository traits**.

## Layering

```text
Application domain (product crate)
    → application repository traits (product-defined)
    → serenade-contracts (UnitOfWork, PageRequest, PersistenceError, …)
        → SQLx adapter (hand-written queries)
        → SeaORM adapter (entities + migrations mirror)
```

Kernel and HTTP crates never depend on `sqlx`, `sea-orm`, or `diesel`.

## Framework traits (`serenade-contracts`)

| Trait / type | Responsibility |
| --- | --- |
| `UnitOfWork` | `begin` / `commit` / `rollback` transaction boundary |
| `PageRequest` | Limit/offset pagination for list reads |
| `PersistenceError` / `RepositoryError` | Shared adapter error surface |
| `EntityId` | Marker for identifiers passed into application ports |
| persist-param helpers | Reject NUL in string parameters at the persistence boundary |

**Domain ports** (`ProductRepository`, cart ports, CMS ports, …) live in the **application**. Serenade does not ship catalog, cart, or order repository traits.

Associated entity types are defined in the application. Serenade stays ORM-agnostic.

## Business rules vs persistence hooks

Serenade does **not** provide a global ORM `preSave` / lifecycle callback. It does not own SeaORM, SQLx, or Diesel entities, so it cannot intercept every database write.

Put behavior in the right layer:

| Concern | Where it belongs | Examples |
| --- | --- | --- |
| **Business / domain rules** | Application domain or use-case service, **before** calling `Repository::save` | Aggregate invariants, stock check, price snapshot |
| **Technical persistence** | Application adapter (or DB trigger) | `updated_at`, soft-delete flags, ORM `before_save` |

Typical flow:

```text
HTTP / console
  → domain service (enforce business rules, mutate aggregate)
    → repository.save / UnitOfWork (adapter writes SQL or ORM)
```

ORM-specific hooks (for example SeaORM `ActiveModelBehavior::before_save`) are valid for **technical** fields when the application chose that ORM. They are not a substitute for domain rules, and they are not part of Serenade core.

Older stacks often attached `preSave` to the ORM model. Same idea: hooks live with the persistence choice the application made; the framework kernel stays ORM-agnostic.

## Errors

`PersistenceError` covers `NotFound`, `Conflict`, `InvalidInput`, and `Internal`. Adapters map driver errors into these variants; HTTP layers map them to status codes.

## Adapter rules

1. **One logical schema** per product. SQLx migrations are canonical; SeaORM migrations mirror them when both adapters exist.
2. **Money** as integer minor units + ISO currency code in the database when the product deals in money. Never floats.
3. **Snapshots** on mutable line items (unit price, labels) at mutation time when the product needs historical accuracy.
4. **Idempotency** for checkout or other once-only writes belongs in the application repository / use-case layer.
5. Integration tests run against Docker Postgres in the application repo.
6. Run **persist-param hygiene** on request/domain strings before bind/filter (see below).

## SQL and parameter hygiene

**SQL injection** is prevented by **parameterized queries** and query builders only. Do not build SQL with `format!` or string concat using user or request data. That path is not supported. SeaORM does not change this: still bind/filter with parameters, never interpolate client text into SQL.

### Persist-param check (`serenade-contracts`)

`reject_unsafe_sql_param` / `PersistParamPolicy` enforce a **persistence boundary invariant**: reject **NUL (`\0`)** in string parameters. Rationale is input hygiene and C / driver / interop safety (NUL-terminated buffers), **not** injection defense. Tab, LF, CR, and other non-NUL bytes remain allowed.

**On by default.** To deliberately take the risk and disable checks:

- process env `SERENADE_DISABLE_PERSIST_PARAM_CHECK=1` (also `true` / `yes` / `on`), or
- `PersistParamPolicy::disabled()` in code

(`SERENADE_DISABLE_SQL_SAFETY` is still honored as a legacy alias.)

Disabling is an explicit risk acceptance, not a normal production setting.

### Raw SQL with client fragments

Any API that runs SQL text (or fragments) supplied by a client must be behind a product config flag that defaults to **off**. Without that flag, refuse the call. Static migrations and embedded seed SQL are not client fragments.

## Mock implementations

`serenade-contracts` tests include an in-memory `UnitOfWork` mock proving the framework traits compile without a database. Application repos should add Postgres integration tests behind CI service containers, and own mocks for their domain repository traits.

## Non-goals (Serenade)

- Migration runners
- Entity definitions for application aggregates
- Domain-specific repository traits (catalog, cart, CMS, …)
- Choosing SQLx vs SeaORM for applications
- Global ORM lifecycle callbacks (`preSave` / `preUpdate` across every driver)
