use std::sync::OnceLock;
use tokio::runtime::Runtime;
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn get_async_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio Runtime")
    })
}
