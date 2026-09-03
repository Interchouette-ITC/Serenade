# Persistence adapters

Serenade defines **contracts**; applications own schema, migrations, and ORM/SQL choices.

## Layering

```text
Application domain (product crate)
    → serenade-contracts traits (ProductRepository, UnitOfWork, …)
        → SQLx adapter (hand-written queries)
        → SeaORM adapter (entities + migrations mirror)
```

Kernel and HTTP crates never depend on `sqlx`, `sea-orm`, or `diesel`.

## Repository traits (`serenade-contracts`)

| Trait | Responsibility |
| --- | --- |
| `ProductRepository` | Read by id, slug, paginated list |
| `CategoryRepository` | Read by id, slug, children of parent |
| `CartRepository` | Find by session token, save, delete |
| `OrderRepository` | Find by number, save, idempotent checkout save |
| `UnitOfWork` | `begin` / `commit` / `rollback` transaction boundary |

Associated types (`Id`, `Product`, `Cart`, …) are defined in the **application**. Serenade stays ORM-agnostic.

## Errors

`PersistenceError` covers `NotFound`, `Conflict`, `InvalidInput`, and `Internal`. Adapters map driver errors into these variants; HTTP layers map them to status codes.

## Adapter rules

1. **One logical schema** per product. SQLx migrations are canonical; SeaORM migrations mirror them.
2. **Money** as integer minor units + ISO currency code in the database. Never floats.
3. **Snapshots** on cart and order lines (unit price, labels) at mutation time.
4. **Idempotency** on checkout via `OrderRepository::save_idempotent`.
5. Integration tests run against Docker Postgres in the application repo.
6. **SQL safety** (see below) before binding or filtering on request/domain strings.

## SQL safety (injection defense-in-depth)

Parameterized queries / query builders are **mandatory**. Do not build SQL with `format!` or string concat using user or request data. That is not a supported path.

SeaORM (or any ORM) does **not** waive these rules: filter values still go through the same checks.

### Parameter guards (`serenade-contracts`)

Call `reject_unsafe_sql_param` / `SqlSafetyPolicy::reject_param` on strings before `.bind` / `.eq`. The guard rejects NUL and other C0 controls (except tab / LF / CR).

**On by default.** To deliberately take the risk and disable checks:

- process env `SERENADE_DISABLE_SQL_SAFETY=1` (also `true` / `yes` / `on`), or
- `SqlSafetyPolicy::disabled()` in code

Disabling is an explicit risk acceptance, not a normal production setting.

### Raw SQL with client fragments

Any API that runs SQL text (or fragments) supplied by a client must be behind a product config flag that defaults to **off**. Without that flag, refuse the call. Static migrations and embedded seed SQL are not client fragments.

## Mock implementations

`serenade-contracts` tests include an in-memory mock proving the traits compile without a database. Application repos should add Postgres integration tests behind CI service containers.

## Non-goals (Serenade)

- Migration runners
- Entity definitions for commerce aggregates
- Choosing SQLx vs SeaORM for applications
