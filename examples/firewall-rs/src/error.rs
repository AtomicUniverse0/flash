#[derive(Debug, thiserror::Error)]
#[error("app error: {0}")]
pub enum AppError {
    Flash(#[from] flash::FlashError),
    Csv(#[from] csv::Error),

    #[cfg(feature = "stats")]
    Tui(#[from] flash::tui::TuiError),

    #[cfg(not(feature = "stats"))]
    #[error("app error: error setting Ctrl-C handler: {0}")]
    Ctrl(#[from] ctrlc::Error),
}
