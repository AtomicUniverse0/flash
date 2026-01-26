#[derive(Debug, thiserror::Error)]
#[error("app error: {0}")]
pub enum AppError {
    Flash(#[from] flash::FlashError),

    #[cfg(feature = "stats")]
    Tui(#[from] flash::tui::TuiError),

    #[cfg(not(feature = "stats"))]
    #[error("app error: error setting Ctrl-C handler: {0}")]
    Ctrl(#[from] ctrlc::Error),

    #[error("app error: empty route and no fallback IP")]
    EmptyRoute,

    #[error(
        "app error: no of next NF MACs ({mac_count}) does not match no of next NFs ({ip_count})"
    )]
    MacIpMismatch {
        mac_count: usize,
        ip_count: usize,
    },
}
