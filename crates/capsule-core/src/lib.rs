//! Headless Capsule journal behavior shared by desktop and non-desktop clients.
//!
//! The crate deliberately has no Tauri, WebView, tray, window, shell, or
//! process-lifecycle dependencies.  Desktop-only actions remain adapters in
//! the Tauri application.

pub mod backup;
pub mod context;
pub mod contracts;
pub mod db;
pub mod entries;
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
pub use contracts::{
    CaptureOutcome, CaptureRequest, CommitReceipt, ContextAttachment, ContextLocation,
    ContextPolicy, ContextResult, ContextStatus, WeatherObservation,
};
pub use db::{
    capability_report_for_database, inspect_capabilities, resolve_capsule,
    resolve_capsule_from_environment, resolve_capsule_with_environment, CapabilityReport,
    CapsuleResolver, FileIdentity, PathDiagnostic, PathSource, ResolveRequest, ResolvedCapsule,
    ResolverEnvironment, SafePathSettings, SchemaCapabilities,
};
pub use location::{load_context_settings, ContextSettings};
