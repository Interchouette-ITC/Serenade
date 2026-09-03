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

## Mock implementations

`serenade-contracts` tests include an in-memory mock proving the traits compile without a database. Application repos should add Postgres integration tests behind CI service containers.

## Non-goals (Serenade)

- Migration runners
- Entity definitions for commerce aggregates
- Choosing SQLx vs SeaORM for applications
