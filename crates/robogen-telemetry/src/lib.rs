pub type InitError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub fn init() -> Result<(), InitError> {
    tracing_subscriber::fmt().with_target(false).try_init()
}
