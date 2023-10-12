use spin_core::{async_trait, HostComponent};
use spin_world::wasi::logging::logging::{self, Level};

#[derive(Default)]
pub struct LoggingComponent;

impl HostComponent for LoggingComponent {
    type Data = Logging;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        spin_world::wasi::logging::logging::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

#[derive(Default)]
pub struct Logging {}

#[async_trait]
impl logging::Host for Logging {
    async fn log(&mut self, level: Level, context: String, message: String) -> anyhow::Result<()> {
        log::log!(
            match level {
                Level::Trace => log::Level::Trace,
                Level::Debug => log::Level::Debug,
                Level::Info => log::Level::Info,
                Level::Warn => log::Level::Warn,
                Level::Critical => log::Level::Error,
                Level::Error => log::Level::Error,
            },
            "{}: {}",
            context,
            message
        );
        Ok(())
    }
}
