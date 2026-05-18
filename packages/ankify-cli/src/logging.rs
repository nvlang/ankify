//! Process-wide hook for raising log verbosity at runtime.
//!
//! The binary owns tracing setup, but the document's `verbose` setting is only
//! discovered partway through a sync. The binary registers a hook here that
//! raises the log level; `sync` calls it once it has read the configuration.

use std::sync::OnceLock;

/// A hook that raises the global log verbosity to debug level.
type VerboseHook = Box<dyn Fn() + Send + Sync>;

static VERBOSE_HOOK: OnceLock<VerboseHook> = OnceLock::new();

/// Register the verbosity-raising hook. Called once, by the binary, after it
/// has set up tracing.
pub fn register_verbose_hook(hook: VerboseHook) {
    let _ = VERBOSE_HOOK.set(hook);
}

/// Raise log verbosity to debug level, if a hook has been registered. A no-op
/// for a library consumer that never registered one.
pub fn enable_verbose_logging() {
    if let Some(hook) = VERBOSE_HOOK.get() {
        hook();
    }
}
