use clap::Parser;
use flash::FlashConfig;

fn main() {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt::init();

    let flash_config = FlashConfig::parse();

    if let Err(err) = flash::connect(&flash_config) {
        eprintln!("{err}");
    }
}
