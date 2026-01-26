pub(crate) type UtilResult<T> = Result<T, UtilError>;

#[derive(Debug, thiserror::Error)]
#[error("util error: {0}")]
pub enum UtilError {
    #[error("util error: no CPU cores found")]
    NoCpuCores,

    #[error("util error: invalid CPU core/range {0}")]
    InvalidCpuCoreRange(String),

    #[error("util error: CPU core {0} not found")]
    CpuCoreNotFound(usize),
}
