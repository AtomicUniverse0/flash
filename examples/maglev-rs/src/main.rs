mod cli;
mod error;
mod maglev;
mod nf;

use std::{
    hash::BuildHasher,
    net::Ipv4Addr,
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use clap::Parser;
use flash::Socket;
use fnv::FnvBuildHasher;
use macaddr::MacAddr6;

#[cfg(feature = "stats")]
use flash::tui::StatsDashboard;

use crate::{cli::Cli, error::AppError, maglev::Maglev};

const MAGLEV_TABLE_SIZE: usize = 65537;

fn socket_thread<H: BuildHasher + Default>(
    mut socket: Socket,
    maglev: &Arc<Maglev<H>>,
    next_ip: &Arc<Vec<Ipv4Addr>>,
    next_mac: &Arc<Vec<MacAddr6>>,
    stop: &Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        if !socket.poll().is_ok_and(|val| val) {
            continue;
        }

        let Ok(descs) = socket.recv() else {
            continue;
        };

        let mut descs_send = Vec::with_capacity(descs.len());
        let mut descs_drop = Vec::with_capacity(descs.len());

        for mut desc in descs {
            if let Ok(pkt) = socket.read_exact(&desc)
                && let Some(idx) = nf::load_balance(pkt, maglev, next_ip)
            {
                if let Some(next_mac) = next_mac.get(idx).or_else(|| next_mac.first()) {
                    pkt[0..6].copy_from_slice(next_mac.as_bytes());
                }

                desc.set_next(idx);
                descs_send.push(desc);
            } else {
                descs_drop.push(desc);
            }
        }

        socket.send(descs_send);
        socket.drop(descs_drop);
    }
}

fn run(mut cli: Cli) -> Result<(), AppError> {
    let (sockets, mut monitor) = flash::connect(&cli.flash_config)?;

    let mut next_ip_addr = monitor.get_next_ip_addr()?;
    if next_ip_addr.is_empty() {
        if let Some(fb_ip) = cli.fallback_ip {
            next_ip_addr.push(fb_ip);
        } else {
            return Err(AppError::EmptyRoute);
        }
    }

    if cli.next_mac.len() > 1 && cli.next_mac.len() != next_ip_addr.len() {
        return Err(AppError::MacIpMismatch {
            mac_count: cli.next_mac.len(),
            ip_count: next_ip_addr.len(),
        });
    }

    let maglev = Arc::new(Maglev::<FnvBuildHasher>::new(
        &next_ip_addr,
        MAGLEV_TABLE_SIZE,
    ));
    let next_ip = Arc::new(next_ip_addr);
    let next_mac = Arc::new(cli.next_mac);

    let stop = Arc::new(AtomicBool::new(true));

    #[cfg(feature = "stats")]
    let mut tui = StatsDashboard::new(
        sockets.iter().map(Socket::stats),
        cli.stats.fps,
        cli.stats.layout,
        Some(stop.clone()),
    )?;

    #[cfg(feature = "stats")]
    let stats_thread = cli.stats.cpu.spawn(move || {
        if let Err(err) = tui.run() {
            eprintln!("error dumping stats: {err}");
        }
    });

    #[cfg(not(feature = "stats"))]
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || {
            stop.store(true, Ordering::Release);
        })
    }?;

    let _ = {
        let stop = stop.clone();
        monitor.spawn_disconnect_handler(move || {
            stop.store(true, Ordering::Release);
        })
    };

    let socket_threads = cli
        .cpu_range
        .spawn_multiple(sockets.into_iter().map(|socket| {
            let stop = stop.clone();
            let maglev = maglev.clone();
            let next_ip = next_ip.clone();
            let next_mac = next_mac.clone();

            move || socket_thread(socket, &maglev, &next_ip, &next_mac, &stop)
        }));

    #[cfg(feature = "stats")]
    if let Err(err) = stats_thread.join() {
        eprintln!("error in stats thread: {err:?}");
    }

    #[cfg(feature = "stats")]
    stop.store(true, Ordering::Release);

    for handle in socket_threads {
        if let Err(err) = handle.join() {
            eprintln!("error in socket thread: {err:?}");
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
