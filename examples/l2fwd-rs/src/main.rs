mod cli;
mod error;

use std::{
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use clap::Parser;
use flash::Socket;
use macaddr::MacAddr6;

#[cfg(feature = "stats")]
use flash::tui::StatsDashboard;

use crate::{cli::Cli, error::AppError};

#[forbid(clippy::indexing_slicing)]
#[inline]
fn mac_swap(pkt: &mut [u8; 14], mac_addr: Option<MacAddr6>) {
    let mut tmp = [0; 6];
    tmp.copy_from_slice(&pkt[0..6]);

    if let Some(mac_addr) = mac_addr {
        pkt[6..12].copy_from_slice(&tmp);
        pkt[0..6].copy_from_slice(mac_addr.as_bytes());
    } else {
        pkt[6..12].swap_with_slice(&mut tmp);
        pkt[0..6].copy_from_slice(&tmp);
    }
}

fn socket_thread(mut socket: Socket, mac_addr: Option<MacAddr6>, stop: &Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        if !socket.poll().is_ok_and(|val| val) {
            continue;
        }

        let Ok(descs) = socket.recv() else {
            continue;
        };

        let mut descs_send = Vec::with_capacity(descs.len());
        let mut descs_drop = Vec::with_capacity(descs.len());

        for desc in descs {
            let Ok(pkt) = socket.read_exact(&desc) else {
                descs_drop.push(desc);
                continue;
            };

            mac_swap(pkt, mac_addr);
            descs_send.push(desc);
        }

        socket.send(descs_send);
        socket.drop(descs_drop);
    }
}

fn run(mut cli: Cli) -> Result<(), AppError> {
    let (sockets, monitor) = flash::connect(&cli.flash_config)?;
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
            move || socket_thread(socket, cli.mac_addr, &stop)
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
