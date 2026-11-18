//! # Serenade
//!
//! Serenade is a Symfony-oriented application framework for Rust: a kernel with
//! bundles, a dependency-injection container, an event dispatcher, layered
//! configuration, and optional components (HTTP, security, messaging, views,
//! forms, ...) that applications compose.
//!
//! `serenade-framework` is the public entry point for normal Serenade consumers.
//! It re-exports a curated, stable surface of the `serenade-*` implementation
//! crates. Those crates stay independently published and usable; the facade
//! never owns their implementation.
//!
//! Website: <https://serenade.rs>. Guides: <https://docs.serenade.rs>.
//!
//! ```toml
//! [dependencies]
//! serenade-framework = "1.0.0"
//! ```
//!
//! The executable named `serenade` is shipped by the separate `serenade-cli`
//! package. The crates.io package `serenade` is unrelated to this project.
//!
//! ## Features
//!
//! The default build contains only the framework foundations ([`kernel`],
//! [`config`], [`di`], [`event`]). It needs no web server, database, Leptos,
//! or Rangular.
//!
//! | Feature | Module | Enables |
//! | --- | --- | --- |
//! | *(default)* | [`kernel`], [`config`], [`di`], [`event`] | Foundations |
//! | `console` | `console` | `serenade-console` |
//! | `bundle` | `bundle` | `serenade-bundle` |
//! | `http` | `http`, `http::client` | `serenade-http`, `serenade-http-client` |
//! | `http-actix` | `http::actix` | `http` + `serenade-http-actix` |
//! | `http-axum` | `http::axum` | `http` + `serenade-http-axum` |
//! | `http-client-reqwest` | `http::client::ReqwestHttpClient` | Reqwest outbound client |
//! | `session` | `session` | `serenade-session` |
//! | `security` | `security` | `serenade-security` |
//! | `security-oauth` / `security-ldap` | `security` | OAuth / LDAP helpers |
//! | `validation` | `validator` | `serenade-validator` |
//! | `lock` | `lock` | `serenade-lock` |
//! | `rate-limiter` | `rate_limiter` | `serenade-rate-limiter` |
//! | `scheduler` | `scheduler` | `serenade-scheduler` |
//! | `scheduler-messenger` | `scheduler` | Scheduler dispatch through the message bus |
//! | `messenger` | `messenger` | `serenade-messenger` |
//! | `mailer` | `mailer` | `serenade-mailer` (no transport backend) |
//! | `mailer-smtp` / `mailer-esp` | `mailer` | SMTP / ESP transports |
//! | `notifier` | `notifier` | `serenade-notifier` |
//! | `view` | `view` | `serenade-view` (engine neutral) |
//! | `view-leptos-ssr` / `view-leptos-client` | `view` | Leptos SSR / hydrate support |
//! | `form` | `form` | `view` + `serenade-form` |
//!
//! Integration features (`*-reqwest`, `*-smtp`, `*-esp`, `*-oauth`, `*-ldap`,
//! `view-leptos-*`, `scheduler-messenger`) are never implied by their base
//! feature.
//!
//! Crates outside this surface (for example `serenade-cache`,
//! `serenade-workflow`, `serenade-translation`, `serenade-testing`) are used
//! directly by depending on them.
//!
//! ## Example
//!
//! ```
//! use serenade_framework::prelude::*;
//!
//! struct Shop;
//!
//! impl BundleInterface for Shop {
//!     fn name(&self) -> &'static str {
//!         "shop"
//!     }
//! }
//!
//! let mut app = App::new(Environment::Test);
//! app.register_bundle(Shop).expect("register");
//! app.boot().expect("boot");
//! assert_eq!(app.kernel().bundle_names(), vec!["shop"]);
//! app.shutdown().expect("shutdown");
//! ```

/// Facade crate version (matches every `serenade-*` crate in the same release).
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Application kernel, bundles, environment, and lifecycle.
pub mod kernel {
    pub use serenade_kernel::{
        App, Application, Bundle, BundleInterface, BundleRegistry, Environment, Kernel,
        KernelError, KernelPhase,
    };
}

/// Layered configuration: dotenv, TOML/YAML packages, env overlays.
pub mod config {
    pub use serenade_config::{
        Config, ConfigError, load_dotenv, load_packages, load_packages_for_env,
    };
}

/// Dependency injection container.
pub mod di {
    pub use serenade_di::{
        CompilePass, Container, ContainerBuilder, DiError, ParameterBag, Reference, Scope,
        ServiceDefinition, ServiceFactory,
    };
}

/// Synchronous event dispatcher.
pub mod event {
    pub use serenade_event::{
        DISPATCHER_SERVICE, Event, EventDispatcher, EventError, EventSubscriber,
        RecordingSubscriber, RegisterEventSubscribersPass, SUBSCRIBER_TAG, SubscriberService,
        assert_dispatched,
    };
}

/// Console application (`bin/console` analogue).
#[cfg(feature = "console")]
pub mod console {
    pub use serenade_console::{
        APPLICATION_SERVICE, AboutCommand, Application, COMMAND_TAG, Command, CommandService,
        ConsoleError, DebugConfigCommand, DebugContainerCommand, Input, RegisterCommandsPass,
    };
}

/// Bundle composition and framework bundle.
#[cfg(feature = "bundle")]
pub mod bundle {
    pub use serenade_bundle::{
        BundleError, CONFIG_SERVICE, Extension, FRAMEWORK_BUNDLE, FrameworkBundle,
        FrameworkExtension, ROUTER_SERVICE, build_container,
    };
}

/// Framework-agnostic HTTP foundation: request, response, routing, middleware, kernel.
#[cfg(feature = "http")]
pub mod http {
    pub use serenade_http::{
        AsyncFn, AsyncHealthMiddleware, AsyncHttpKernel, AsyncMiddleware, AsyncNext,
        AsyncRequestHandler, AsyncRequestIdMiddleware, AttributeBag, BoxFuture,
        DefaultExceptionHandler, ExceptionHandler, HEALTHZ_PATH, Headers, HealthMiddleware,
        HttpCacheHeaders, HttpError, HttpKernel, MatchResult, Method, Middleware, READYZ_PATH,
        REQUEST_ID_ATTRIBUTE, REQUEST_ID_HEADER, ROUTE_ATTRIBUTE, Readiness, Request,
        RequestHandler, RequestIdMiddleware, Response, Route, RouteCollection, RouteLoader,
        SyncToAsync, UrlMatcher, box_future, ensure_request_id, etag, healthz, load_routes,
        maybe_not_modified, readyz, request_id, weak_etag,
    };

    /// Outbound HTTP client contract.
    pub mod client {
        #[cfg(feature = "http-client-reqwest")]
        pub use serenade_http_client::ReqwestHttpClient;
        pub use serenade_http_client::{
            ClientMethod, ClientRequest, ClientResponse, DEFAULT_HTTP_CLIENT_SERVICE,
            DynHttpClient, HTTP_CLIENT_TAG, HttpClient, HttpClientError, HttpClientService,
            MockHttpClient, RegisterDefaultHttpClientPass,
        };
    }

    /// Actix-web bridge.
    #[cfg(feature = "http-actix")]
    pub mod actix {
        pub use serenade_http_actix::{
            app, await_bound, bind_server, conversion_error, dispatch, dispatch_async, from_actix,
            listen, to_actix,
        };
    }

    /// Axum bridge.
    #[cfg(feature = "http-axum")]
    pub mod axum {
        pub use serenade_http_axum::{
            BoundServer, ShutdownHandle, await_bound, bind_server, conversion_error, dispatch,
            dispatch_async, from_axum, listen, router, to_axum,
        };
    }
}

/// Session bag, stores, flash messages, and middleware.
#[cfg(feature = "session")]
pub mod session {
    pub use serenade_session::{
        AsyncSessionMiddleware, CookieSession, CookieSessionOptions, DEFAULT_SESSION_COOKIE,
        FLASH_SESSION_KEY, FlashBag, MemorySessionStore, SESSION_ATTRIBUTE, SameSite, Session,
        SessionError, SessionMiddleware, SessionStore, generate_session_id, parse_cookie_value,
        request_session, request_session_mut,
    };
}

/// Authentication, authorization, CSRF, and password hashing.
#[cfg(feature = "security")]
pub mod security {
    pub use serenade_security::{
        AccessDecisionManager, Argon2idPasswordHasher, AsyncSessionTokenMiddleware, Authenticator,
        CSRF_FIELD_NAME, CsrfToken, CsrfTokenManager, FirewallMiddleware, HmacCsrfTokenManager,
        InMemoryUser, PasswordHasher, RoleVoter, SECURITY_SESSION_KEY, SecurityError,
        SessionTokenMiddleware, Subject, TOKEN_ATTRIBUTE, TokenInterface, UserInterface,
        UsernamePasswordToken, Vote, Voter, login, logout, request_token, token_from_session,
    };
    #[cfg(feature = "security-oauth")]
    pub use serenade_security::{
        AuthorizationRequest, MockTokenExchanger, OAuthClientConfig, OAuthTokenResponse,
        TokenExchanger, build_authorization_request, build_authorization_request_with,
        decode_jwt_payload_json, parse_token_response, subject_from_id_token, token_exchange_form,
        token_from_oidc_subject,
    };
    #[cfg(feature = "security-ldap")]
    pub use serenade_security::{
        LDAP_USERNAME_PLACEHOLDER, LdapAuthenticator, LdapBindConfig, LdapBinder, LdapIdentity,
        MockLdapBinder, authenticate_ldap_password, split_user_password, token_from_ldap_identity,
    };
}

/// Constraint validation for DTOs and commands.
#[cfg(feature = "validation")]
pub mod validator {
    pub use serenade_validator::{
        Constraint, ConstraintViolationList, Length, MessengerValidateHook, NotBlank, Range,
        RecursiveValidator, Validatable, Validator, ValidatorError, Violation,
    };
}

/// Named locks.
#[cfg(feature = "lock")]
pub mod lock {
    pub use serenade_lock::{
        DEFAULT_LOCK_STORE_SERVICE, DEFAULT_LOCK_TTL, FilesystemLockStore,
        FilesystemLockStoreConfig, InMemoryLockStore, LOCK_STORE_TAG, Lock, LockError, LockFactory,
        LockKey, LockStore, LockStoreService, RegisterDefaultLockPass, generate_lock_token,
        validate_resource,
    };
}

/// Rate limiting.
#[cfg(feature = "rate-limiter")]
pub mod rate_limiter {
    pub use serenade_rate_limiter::{
        ConsumeOrExceedError, DEFAULT_RATE_LIMITER_STORAGE_SERVICE, InMemoryRateLimiterStorage,
        Policy, RATE_LIMITER_STORAGE_TAG, RateLimit, RateLimitExceeded, RateLimiter,
        RateLimiterError, RateLimiterFactory, RateLimiterStorage, RateLimiterStorageService,
        RegisterDefaultRateLimiterPass, WindowState, consume_or_exceed, require_accepted,
        too_many_requests, validate_key,
    };
}

/// Scheduling.
#[cfg(feature = "scheduler")]
pub mod scheduler {
    #[cfg(feature = "scheduler-messenger")]
    pub use serenade_scheduler::CommandOnDue;
    pub use serenade_scheduler::{
        Clock, DEFAULT_SCHEDULER_SERVICE, DueJob, FnScheduleHandler, HandlerMap, ManualClock,
        RegisterDefaultSchedulerPass, RunSchedulerCommand, SCHEDULER_TAG, Schedule,
        ScheduleHandler, Scheduler, SchedulerError, SchedulerService, SystemClock, Trigger,
        validate_id,
    };
}

/// Command and event message bus.
#[cfg(feature = "messenger")]
pub mod messenger {
    pub use serenade_messenger::{
        Command, CommandHandler, DispatchContext, DispatchKind, Envelope, Event, EventHandler,
        InMemoryTransport, LogSink, LoggingMiddleware, Message, MessageBus, MessengerError,
        Middleware, Transport, ValidateHook, ValidationMiddleware, WireEnvelope,
    };
}

/// Email messages and transports.
#[cfg(feature = "mailer")]
pub mod mailer {
    pub use serenade_mailer::{
        Address, Attachment, Body, ContentDisposition, DEFAULT_MAILER_SERVICE, Email,
        FileTransport, MAILER_TRANSPORT_TAG, MailerError, MailerService, MimePart, MimeTree,
        NullTransport, RegisterDefaultMailerPass, Transport,
    };
    #[cfg(feature = "mailer-esp")]
    pub use serenade_mailer::{
        EspApiConfig, EspAuthScheme, EspHttpPoster, EspHttpResponse, EspHttpTransport,
        MockEspHttpPoster, MockEspRequest, build_esp_payload,
    };
    #[cfg(feature = "mailer-smtp")]
    pub use serenade_mailer::{SmtpTransport, SmtpTransportBuilder};
}

/// SMS and push notifications.
#[cfg(feature = "notifier")]
pub mod notifier {
    pub use serenade_notifier::{
        Channel, DEFAULT_NOTIFIER_SERVICE, MemoryTransport, NOTIFIER_TRANSPORT_TAG, Notification,
        NotifierError, NotifierService, NullTransport, PushMessage, RegisterDefaultNotifierPass,
        SmsMessage, SmsOnlyTransport, Transport,
    };
}

/// HTML view helpers and document composition (template-engine neutral).
#[cfg(feature = "view")]
pub mod view {
    pub use serenade_view::{
        AssetConfig, DefaultDocumentTemplate, Document, DocumentAssets, DocumentParts,
        DocumentTemplate, RenderedDocument, TRUNK_ASSET_MANIFEST_ENV, TRUNK_ASSET_MANIFEST_FILE,
        TRUNK_DIST_DIRECTORY_ENV, TrunkAssetManifest, TrunkAssetReference, TrunkDocumentAsset,
        TrunkPreloadReference, TrunkWasmBundle, ViewError, asset, escape_attr, escape_html,
        partial, path,
    };
    #[cfg(feature = "view-leptos-ssr")]
    pub use serenade_view::{DocumentSlot, LeptosExecutor, initialize_leptos_ssr};
    #[cfg(any(feature = "view-leptos-client", feature = "view-leptos-ssr"))]
    pub use serenade_view::{JsonLd, ManagedHead, ManagedLink, ManagedMeta, ManagedTitle};
    #[cfg(not(target_arch = "wasm32"))]
    pub use serenade_view::{STATIC_OUTPUT_DIR_ENV, StaticDocumentWriter};
}

/// HTML forms: binding, CSRF, rendering, multipart.
#[cfg(feature = "form")]
pub mod form {
    pub use serenade_form::{
        BoolToStringTransformer, COLLECTION_PROTOTYPE_INDEX, CSRF_FIELD_NAME, CheckboxOptions,
        ChoiceEntry, ChoiceOptions, ChoiceToValueTransformer, CollectionEntryBuilder,
        CollectionPrototypeMeta, CompoundBuilder, DEFAULT_MAX_FILE_SIZE,
        DEFAULT_MAX_MULTIPART_BODY, DataTransformer, Field, FieldConfig, FieldKind, FileOptions,
        FileStorage, Form, FormBuilder, FormError, FormStatus, HiddenOptions,
        I64ToStringTransformer, MultipartData, NestedData, NumberOptions, PasswordOptions,
        RenderedForm, TextOptions, TextareaOptions, UploadedFile, collection_field_name,
        collection_indices, collection_prototype_field_name, escape_attr, escape_html, form_csrf,
        form_errors, form_label, form_row, form_widget, is_multipart, join_multi,
        multipart_boundary, parse_form_body, parse_multipart, parse_urlencoded,
        parse_urlencoded_multi, split_multi, unflatten_form_data,
    };
}

/// Common imports for application code built on the default feature set.
pub mod prelude {
    pub use crate::config::Config;
    pub use crate::di::{Container, ContainerBuilder, ServiceDefinition};
    pub use crate::event::{Event, EventDispatcher, EventSubscriber};
    pub use crate::kernel::{App, Application, BundleInterface, Environment, Kernel, KernelPhase};
}
