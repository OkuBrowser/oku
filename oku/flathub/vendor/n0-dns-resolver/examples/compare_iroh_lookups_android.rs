//! Runs [`compare_iroh_lookups`] on an Android device, with the JNI path to the
//! system's DNS servers live.
//!
//! ```text
//! cargo apk run --example compare_iroh_lookups_android
//! ```
//!
//! `cargo-apk` builds a NativeActivity out of a `cdylib`, and an example cannot
//! be both that and a runnable binary, so this is a separate target rather than
//! a `cfg` block: it compiles the example next door as a module and calls its
//! `run`.
//!
//! [`compare_iroh_lookups`]: ../compare_iroh_lookups.rs

/// Android discards a process's stdout, so send the example's `println!` to
/// logcat instead. Declared before the module, which is what puts it in scope
/// inside it.
#[cfg(target_os = "android")]
macro_rules! println {
    () => { tracing::info!("") };
    ($($arg:tt)*) => { tracing::info!($($arg)*) };
}

// Its `main` goes unused: this target enters at `android_main` below.
#[cfg(target_os = "android")]
#[allow(dead_code)]
#[path = "compare_iroh_lookups.rs"]
mod compare_iroh_lookups;

/// The NativeActivity entry point.
///
/// `android-activity` initializes [`ndk_context`] before calling this, which is
/// the whole point of running here: it is what lets both resolvers read the
/// device's DNS servers over JNI. The activity draws nothing, and output goes
/// to logcat, which `cargo apk run` streams.
///
/// [`ndk_context`]: https://docs.rs/ndk-context
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(_app: android_activity::AndroidApp) {
    init_logging();
    compare_iroh_lookups::run()
}

/// Sends the example's output to logcat, and not much else.
///
/// The results are `println!`, which this target turns into `info` events, and
/// the default filter passes those plus any warning. An Android process has no
/// environment to read at run time, so `RUST_LOG` is read at build time
/// instead; changing it rebuilds the example:
///
/// ```text
/// RUST_LOG=n0_dns_resolver=debug cargo apk run --example compare_iroh_lookups_android
/// ```
#[cfg(target_os = "android")]
fn init_logging() {
    use tracing_subscriber::{
        filter::Targets, layer::SubscriberExt as _, util::SubscriberInitExt as _,
    };

    /// Results and warnings. The glue's lifecycle logging is off because it
    /// reports the activity being torn down as errors.
    const DEFAULT_FILTER: &str = "warn,android_activity=off,compare_iroh_lookups_android=info";

    let filter: Targets = option_env!("RUST_LOG")
        .unwrap_or(DEFAULT_FILTER)
        .parse()
        .expect("RUST_LOG parses as a tracing filter");
    tracing_subscriber::registry()
        .with(paranoid_android::layer("compare_iroh_lookups").with_target(false))
        .with(filter)
        .init();
}
