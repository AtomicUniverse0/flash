use std::str::FromStr as _;

use clap::Parser;
use flash::FlashConfig;
use macaddr::MacAddr6;
use utils::CpuRange;

#[cfg(feature = "stats")]
use flash::tui::GridLayout;

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(flatten)]
    pub flash_config: FlashConfig,

    #[arg(
        short = 'c',
        long,
        default_value_t = CpuRange::default(),
        value_parser = CpuRange::from_str,
        help = "CPU core range for socket threads"
    )]
    pub cpu_range: CpuRange,

    #[arg(short = 'm', long, help = "Dest MAC address")]
    pub mac_addr: Option<MacAddr6>,

    #[cfg(feature = "stats")]
    #[command(flatten)]
    pub stats: StatsConfig,
}

#[cfg(feature = "stats")]
#[derive(Debug, Parser)]
pub struct StatsConfig {
    #[arg(
        short = 's',
        long = "stats-cpu",
        help = "CPU core index for stats thread"
    )]
    pub cpu: CpuRange,

    #[arg(short = 'F', long, default_value_t = 1, help = "Tui frames per second")]
    pub fps: u64,

    #[arg(
        short = 'l',
        long,
        default_value_t = GridLayout::default(),
        value_parser = GridLayout::from_str,
        help = "Tui layout"
    )]
    pub layout: GridLayout,
}
