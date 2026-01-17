use std::path::PathBuf;

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
        help = "CPU core range for socket threads"
    )]
    pub cpu_range: CpuRange,

    #[arg(short, long, help = "Path to denylist csv file")]
    pub denylist: PathBuf,

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
        default_value_t = CpuRange::default(),
        help = "CPU core index for stats thread"
    )]
    pub cpu: CpuRange,

    #[arg(short = 'F', long, default_value_t = 1, help = "Tui frames per second")]
    pub fps: u64,

    #[arg(
        short = 'l',
        long,
        default_value_t = GridLayout::default(),
        help = "Tui layout"
    )]
    pub layout: GridLayout,
}
