use anyhow::Result;

use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

pub fn init(package_name: &str) -> Result<()> {
    let default_directive = format!("{}=info", package_name.replace("-", "_"));

    match tracing_journald::layer() {
        Ok(journald_layer) => tracing_subscriber::registry()
            .with(
                EnvFilter::builder()
                    .with_default_directive(default_directive.parse()?)
                    .from_env_lossy(),
            )
            .with(journald_layer)
            .init(),
        Err(_) => tracing_subscriber::registry()
            .with(
                EnvFilter::builder()
                    .with_default_directive(default_directive.parse()?)
                    .from_env_lossy(),
            )
            .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
            .init(),
    };

    Ok(())
}

/// Initialize tracing using the calling crate's package name.
#[macro_export]
macro_rules! init_tracing {
    () => {
        $crate::tracing::init(env!("CARGO_PKG_NAME"))
    };
}
