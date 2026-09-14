//! Headless Capsule journal behavior shared by desktop and non-desktop clients.
//!
//! The crate deliberately has no Tauri, WebView, tray, window, shell, or
//! process-lifecycle dependencies.  Desktop-only actions remain adapters in
//! the Tauri application.

pub mod backup;
pub mod context;
pub mod capture;
pub mod contracts;
pub mod db;
pub mod entries;
pub mod identity;
pub mod location;
pub mod models;
pub mod providers;

pub use context::{
    capture_context, capture_context_with_dependencies, enrich_context,
    enrich_context_with_dependencies, Cancellation, CancellationToken, Clock, ContextCache,
    ContextDeadline, ContextDependencies, ContextDetails, ContextPreparation, ContextReport,
    ContextRequest, ContextService, HttpClient, HttpRequest, HttpResponse, ManualClock,
    MemoryContextCache, NeverCancel, NoopContextCache, ReqwestHttpClient, SystemClock,
    WeatherCacheKey,
};
pub use backup::{
    with_database_backup_for_database_using_policy_with_timeout, with_mutation_lock_for_database,
    with_mutation_lock_for_database_with_timeout,
};
pub use capture::{
    capture_entry, capture_entry_for_database, capture_entry_with_hooks,
    capture_entry_with_hooks_for_database, reconcile_capture_for_database, CaptureError,
    CaptureErrorCode, CaptureHookPoint, CaptureHooks, CaptureResult, CaptureStatus,
};
pub use contracts::{
    BackupPolicy, CaptureOutcome, CaptureRequest, CommitReceipt, ContextAttachment,
    ContextLocation, ContextPolicy, ContextResult, ContextStatus, WeatherObservation,
};
pub use db::{
    capability_report_for_database, inspect_capabilities, resolve_capsule,
    resolve_capsule_from_environment, resolve_capsule_with_environment, CapabilityReport,
    CapsuleResolver, FileIdentity, PathDiagnostic, PathSource, ResolveRequest, ResolvedCapsule,
    ResolverEnvironment, SafePathSettings, SchemaCapabilities,
};
pub use location::{load_context_settings, ContextSettings};
