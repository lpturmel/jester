pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("os error: {0}")]
    Os(#[from] winit::error::OsError),
    #[error("window error: {0}")]
    External(#[from] winit::error::ExternalError),
    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
}
