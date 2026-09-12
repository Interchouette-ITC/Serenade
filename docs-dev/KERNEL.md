# Kernel components

Cross-cutting infrastructure Serenade owns. Products register domain services and bundles on top.

## Component map

| Component | Responsibility |
| --- | --- |
| **Kernel / lifecycle** | Boot, environment, bundle registration order, shutdown |
| **Dependency injection** | Service container, autowiring-style resolution, scoped services |
| **Event dispatcher** | Sync domain and infrastructure events; subscriber tags |
| **HTTP foundation** | Request/response types, attributes, lifecycle (framework-agnostic core) |
| **HTTP kernel** | Middleware pipeline, controller/action resolution |
| **Routing** | Route collection, requirements, method constraints |
| **Configuration** | Layered config (defaults, env, TOML package files; YAML still accepted) |
| **Console** | CLI application (`bin/console` analogue), commands, optional rich TUI |
| **Cache** | PSR-like pools (memory / filesystem / Redis, tags) plus HTTP response cache headers in `serenade-http` |
| **Session** | Session bag, flash, `SessionStore`, cookie session, HTTP middleware (`serenade-session`) |
| **Security** | AuthN/Z hooks, firewall, voters, CSRF token manager |
| **Form** | HTML forms: bind, CSRF by default, XSS-safe render ([FORMS.md](FORMS.md)) |
| **Translation** | Translator, catalogues, locale negotiation, ICU format helpers ([I18N.md](I18N.md)) |
| **Messenger** | Command/query/event bus; async transport adapters |
| **Serializer** | DTO ↔ JSON (and other formats); normalizers |
| **Validator** | Constraint validation on DTOs and commands |
| **Contracts** | Stable traits for persistence, clock, id generation, etc. |
| **Observability** | Structured logging conventions (Monolog-like channels / `var/log` layout); tracing/metrics hooks |
| **Testing** | Kernel test harness, fake container, event assertion helpers |

## Lifecycle

```text
Created  →  register bundles (any order)
         →  compile (sort by dependencies, then `BundleInterface::build`)
         →  boot (`BundleInterface::boot`)
         →  shutdown (`BundleInterface::shutdown`, reverse dependency order)
```

`serenade-kernel` exposes `Kernel`, `App`, `BundleInterface` / `Bundle`, `BundleRegistry`, and `Environment` (`dev` / `test` / `prod`, plus `Custom` for names like `staging` or `recette`). Debug defaults on for `dev` and `test` only. An application with zero bundles is a valid boot.

`Kernel::boot` compiles first when the kernel is still in `Created`. After `Shutdown` the kernel is terminal.

## Dependency injection

`serenade-di` provides `ContainerBuilder` → compile passes → `Container`.

| Concept | Role |
| --- | --- |
| `ServiceDefinition` | Id, `Scope` (singleton / prototype), declared `Reference` dependencies |
| `ParameterBag` | String parameters available to factories |
| `CompilePass` | Extensible pipeline run before the container freezes |
| `Container::get` / `get_as` | Resolve by id or alias; detects circular dependency at compile and resolve |

Factories return `Box<dyn Any + Send + Sync>`; callers downcast with `get_as`.

## Events

`serenade-event` provides a synchronous `EventDispatcher`.

| Concept | Role |
| --- | --- |
| `Event` | Named payload (`cart.updated`, `order.placed`, …) |
| `EventSubscriber` | Handles one event name; higher `priority` runs first |
| `RegisterEventSubscribersPass` | Collects services tagged `event.subscriber` into `event_dispatcher` |
| `RecordingSubscriber` / `assert_dispatched` | Test harness for dispatch order |

Dispatch stays in-process. Async transports belong on the messenger component.

## Request path

`serenade-http` owns framework-agnostic types. Thin server adapters copy bytes in and out.

```text
HTTP adapter (Actix / Axum / …)
    → Request (method, path, headers, body, attributes)
    → UrlMatcher (RouteCollection; writes `_route` + path params)
    → Middleware stack (first registered layer is outermost)
    → Controller (`RequestHandler`)
    → Response
```

`HttpKernel::handle` always returns a `Response`. Handler and middleware errors go through `ExceptionHandler` (`DefaultExceptionHandler` maps `HttpError` status and message to `text/plain`).

`AsyncHttpKernel` is the async counterpart: controllers return a future (database I/O). Wrap sync handlers with `AsyncHttpKernel::from_sync`. Actix `listen` / `app` take `AsyncHttpKernel`; sync `dispatch` remains for `HttpKernel`.

Routing lives in `serenade-http`: `Route` / `RouteCollection`, `UrlMatcher` (404 / 405), and `RouteLoader` for bundles. Path segments `{name}` become request attributes.

## HTTP adapters

Server crates stay thin:

1. Map the server request (method, path, headers, body) into `serenade_http::Request`.
2. Optionally run `UrlMatcher::apply` to set `_route` and path parameters.
3. Call `HttpKernel::handle`.
4. Map `serenade_http::Response` back to the server response type.

`serenade-http-actix` implements that bridge for Actix Web (`from_actix`, `to_actix`, `dispatch` / `dispatch_async`). For skeletons that only need to bind and serve a kernel, call `serenade_http_actix::listen(addr, async_kernel)` (or `app(data)` when composing Actix yourself). An Axum adapter can reuse the same four steps without changing the foundation crate.

## Messenger and jobs

| Pattern | Use |
| --- | --- |
| Command bus | Mutations with one handler |
| Query bus | Read models (optional CQRS) |
| Event bus | Side effects after commit |
| Async transport | Queue/worker integration (product chooses backend) |

Sync in-process dispatch lives in **`serenade-messenger`**: implement `Message` with a stable `NAME`, mark as `Command` or `Event`, register handlers on `MessageBus`, then `dispatch_command` / `dispatch_event`. Commands require exactly one handler; events fan out (missing handlers are a no-op). Register `Middleware` layers with `add_middleware` (first registered is outermost); built-ins include `LoggingMiddleware` + `LogSink` and `ValidationMiddleware` + `ValidateHook`. `DispatchContext` exposes `payload` (`&dyn Any`) so validation hooks can inspect the message. Async backends implement `Transport` (`send` returns a boxed future); `InMemoryTransport` is the in-process adapter for tests and local workers ([#11](https://github.com/Interchouette-ITC/Serenade/issues/11)). Products wire long-running work through messenger transports rather than ad-hoc task spawning.

## Serializer

DTO ↔ wire formats live in **`serenade-serializer`** ([#12](https://github.com/Interchouette-ITC/Serenade/issues/12), TOON [#125](https://github.com/Interchouette-ITC/Serenade/issues/125)). See [SERIALIZER.md](SERIALIZER.md).

| Piece | Role |
| --- | --- |
| `Encoder` / `Decoder` | Intermediate `serde_json::Value` ↔ bytes; `JsonEncoder` / `JsonDecoder` for `json` |
| `ToonEncoder` / `ToonDecoder` / `encode_toon_string` | TOON v4.1 for LLM / agent context (Cargo feature `toon`) |
| `Normalizer` / `Denormalizer` | Object ↔ `Value` for types that need a custom graph walk |
| `NormalizerRegistry` | First matching normalizer/denormalizer wins |
| `Serializer` | Normalize then encode; decode then denormalize (`SerializerInterface` analogue) |
| Serde bridge | `serialize_value` / `deserialize_value` for typed `Serialize` / `DeserializeOwned` JSON |

Custom types register normalizers on the registry. Redis/XML formats remain out of scope. HTTP and MCP stay JSON; TOON is opt-in export for prompts.

## Validator

Constraint checks live in **`serenade-validator`** ([#13](https://github.com/Interchouette-ITC/Serenade/issues/13)).

| Piece | Role |
| --- | --- |
| `Constraint` | Rule that may append a `Violation` (`NotBlank`, `Length`, `Range`) |
| `Validator` / `RecursiveValidator` | Runs constraints into a `ConstraintViolationList` |
| `Validatable` | Object validates itself through a `Validator` |
| `MessengerValidateHook` | Implements messenger `ValidateHook`; rejects with `MessengerError::Rejected` |

Register message types on the hook, then wrap the bus with `ValidationMiddleware`.

## Security

AuthN/Z and CSRF live in **`serenade-security`** ([#10](https://github.com/Interchouette-ITC/Serenade/issues/10), CSRF in [#110](https://github.com/Interchouette-ITC/Serenade/issues/110)). See [SECURITY.md](SECURITY.md).

| Piece | Role |
| --- | --- |
| `UserInterface` / `TokenInterface` | Principal and credentials carrier |
| `Voter` / `AccessDecisionManager` | Affirmative access checks |
| `FirewallMiddleware` | HTTP middleware: header → `Authenticator` → `_security_token` attribute |
| `CsrfTokenManager` / `HmacCsrfTokenManager` | Stateless HMAC CSRF tokens (`_token`) |
| `PasswordHasher` / `Argon2idPasswordHasher` | Argon2id hash/verify (PHC string) |
| `login` / `SessionTokenMiddleware` | Session identity bridge for HTML logins |

OAuth/OIDC is out of scope. Apps plug bearer or API-key authenticators. HTML forms use CSRF via **`serenade-form`**.

## Admin / back office

Serenade does **not** ship a Symfony 1-style **admin generator** (YAML → full CRUD UI) in the kernel or `FrameworkBundle`. That matches modern Symfony: core has no EasyAdmin; ecosystem and apps own the BO.

| Concern | Owner |
| --- | --- |
| Hand-written or SPA admin UIs | App / product (e.g. MyFeed `/admin`, product admin hosts) |
| Form + CSRF + escape on admin pages | Serenade ([FORMS.md](FORMS.md), [SECURITY.md](SECURITY.md)) |
| Firewall / session login for admin routes | Serenade primitives; apps wire authenticators |
| Optional EasyAdmin-shaped Admin CRUD **bundle** | Not in FrameworkBundle; parked scope in [ADMIN.md](ADMIN.md) ([#124](https://github.com/Interchouette-ITC/Serenade/issues/124)) |

Stance and epic: [#122](https://github.com/Interchouette-ITC/Serenade/issues/122). Docs lock: [#123](https://github.com/Interchouette-ITC/Serenade/issues/123). Parked bundle note: [ADMIN.md](ADMIN.md).

## Forms

HTML form helpers live in **`serenade-form`** ([#110](https://github.com/Interchouette-ITC/Serenade/issues/110)). See [FORMS.md](FORMS.md).

| Piece | Role |
| --- | --- |
| `Form` / `FormBuilder` | Fields, bind POST, validate, render |
| CSRF default on | `_token` + `CsrfTokenManager` |
| `escape_html` / `escape_attr` | XSS-safe output |

Apps stay thin: declare fields and constraints; framework owns CSRF and escape.

## View helpers

Optional HTML helpers live in **`serenade-view`** ([#111](https://github.com/Interchouette-ITC/Serenade/issues/111)). See [VIEW.md](VIEW.md).

| Piece | Role |
| --- | --- |
| `path` / `RouteCollection::generate` | Named reverse routing |
| `asset` / `AssetConfig` | Asset URL under a base prefix |
| `partial` | Include-style fragment nesting (no engine) |
| Re-exported escape | Same XSS helpers as Forms |

## Translation / i18n

UI strings and locale live in **`serenade-translation`** ([#132](https://github.com/Interchouette-ITC/Serenade/issues/132)). See [I18N.md](I18N.md).

| Piece | Role |
| --- | --- |
| `Translator` / catalogues | TOML-first domains, fallbacks, `{name}` + ICU plural subset |
| `LocaleNegotiator` | Query / cookie / `Accept-Language` → `_locale` |
| ICU format helpers | Number, currency code, date (feature `icu`) |
| `TranslationExtension` | DI service `translator` |

Product multilang entity rows stay in the application database.

## Cache

PSR-like pools live in **`serenade-cache`** ([#9](https://github.com/Interchouette-ITC/Serenade/issues/9), Redis follow-up [#99](https://github.com/Interchouette-ITC/Serenade/issues/99)). See [CACHE.md](CACHE.md).

| Piece | Role |
| --- | --- |
| `CacheItem` / `ArrayCacheItem` | Key, hit flag, `Arc` value, optional TTL, optional tags |
| `CacheItemPool` / `ArrayAdapter` | In-memory get/save/delete/clear; `invalidate_tags` |
| `FilesystemAdapter` | Disk-backed pool; TTL; marshaller for `String` / `Vec<u8>` |
| `RedisAdapter` (feature `redis`) | redis-rs + r2d2; prefix keys; `SET`/`PX`; SCAN clear |
| `cache.pool` tag | DI tag; `RegisterDefaultCachePoolPass` seeds `cache.app` |
| HTTP headers | `HttpCacheHeaders` / `maybe_not_modified` in `serenade-http` (see [CACHE.md](CACHE.md)) |

Default DI pool is in-memory. Apps register `FilesystemAdapter` or `RedisAdapter` when they need persistence beyond process memory.

## Session

Session stickiness lives in **`serenade-session`**. See [SESSION.md](SESSION.md).

| Piece | Role |
| --- | --- |
| `Session` | Attribute bag for one request |
| `FlashBag` | One-shot messages on the session |
| `SessionStore` / `MemorySessionStore` | Persist attributes by opaque id |
| `CookieSession` | Open/commit with session-id cookie |
| `SessionMiddleware` / `AsyncSessionMiddleware` | Load/save per request on `HttpKernel` / `AsyncHttpKernel` |

Push `SessionMiddleware::new(CookieSession::new(store))` (first registered is outermost). Controllers use `request_session` / `request_session_mut`. CSRF stays in `serenade-security` (HMAC, no session required).

## Search / indexation

Basic document search lives in **`serenade-search`** ([#129](https://github.com/Interchouette-ITC/Serenade/issues/129)). See [SEARCH.md](SEARCH.md).

| Piece | Role |
| --- | --- |
| `SearchDocument` / `SearchQuery` / `SearchHit` | Ids, text fields, ranked hits |
| `DocumentIndex` | Upsert / delete / query / clear |
| `MemorySearchAdapter` | Zero-deps in-memory index |

No SaaS engine in core. Apps wire adapters when they need one.

## Mailer / Mime lite

Outgoing message types live in **`serenade-mailer`** ([#153](https://github.com/Interchouette-ITC/Serenade/issues/153)). See [MAILER.md](MAILER.md).

| Piece | Role |
| --- | --- |
| `Address` | Mailbox (email + optional display name) |
| `Body` | Text and/or HTML parts |
| `Attachment` | Filename, content type, bytes |
| `Email` | Builder for From/To/Cc/Bcc/Reply-To, subject, body, attachments |
| `Transport` | Sync `send` contract |
| `NullTransport` / `FileTransport` / `SmtpTransport` | Discard / file dump / SMTP (lettre) |
| `RegisterDefaultMailerPass` | DI: service `mailer` defaults to null |

`FrameworkExtension` installs the default mailer pass. See [MAILER.md](MAILER.md).

## Observability (structured logging)

`serenade-observability` documents Monolog-like conventions on **`tracing`**: named channels (`serenade::app`, `serenade::request`, …), an app-owned `var/log/{env}.log` layout, and `LoggingConfig` / `init` for stderr + daily rolling files. Apps call `init` from `main`; crates do not require a global logger. See [OBSERVABILITY.md](OBSERVABILITY.md).

The browser Web Debug Toolbar / Profiler lives in **`serenade-profiler`** ([PROFILER.md](PROFILER.md)): per-request collectors, HTML toolbar injection, and `/_profiler/{token}`. It is not the console. `AsyncHttpKernel` supports an async middleware pipeline (`AsyncMiddleware`) so Actix/`listen` apps can push the profiler the same way sync `HttpKernel` pushes `Middleware`.

## Configuration layers

`serenade-config` loads **TOML** and YAML mappings, deep-merges them, interpolates `${VAR}` / `${VAR:-default}`, and flattens dotted keys into the DI `ParameterBag`.

**Preference:** new apps and Serenade examples use `config/packages/*.toml`. YAML remains supported for Symfony familiarity. There is no PHP/XML config track.

```text
load_dotenv(project_root, APP_ENV)
    .env → .env.local (not in prod) → .env.{env} → .env.{env}.local
    (later files override earlier; process env never overwritten)
defaults (TOML or YAML)
    + config/packages/*.{toml,yaml,yml}  (sorted by file name; files only)
    + config/packages/{env}/*.{toml,yaml,yml}  (env overlay when the directory exists)
    + ${VAR} interpolation from the process environment
```

`load_packages` loads the base directory only. `load_packages_for_env` applies the overlay. `build_container(packages_dir, environment, extensions)` uses the env-aware loader. Apps should call `load_dotenv` on the project root before building the container when they ship `.env` files.

Mappings merge; scalars and sequences in an overlay replace the base value. Unset variables without a default fail load.

## Console

The console component is Serenade’s `bin/console` analogue: discoverable commands, `--env` / `--no-debug`, kernel boot for ops (migrate, workers, debug). Plain commands use clap; interactive debug surfaces may use **ratatui**. See [CONSOLE.md](CONSOLE.md). Flex-like app scaffolding is separate (#30); Cargo remains the package manager.

## Extension surface (kernel-level)

- Tagged services (event subscriber, command handler, …)
- Compiler passes / container extensions (bundle `build()` phase)
- Middleware ordering
- Route loaders from bundles

Domain-specific hooks (pricing, shipping) belong in **product bundles**, not the Serenade core.
