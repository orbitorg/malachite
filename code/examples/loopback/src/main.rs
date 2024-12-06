// Demonstrates the simplest way to instantiate the Malakite library.
//
// We will use a purely local instance of Malakite. The approach is to
// simulate everything, the network, signing, mempool, etc.
// Each peer is a simple data structure. Messages passing via direct
// function calls.
//
// The experience of building a system on top of Malakite in this example
// should be no different from building on top of an SQLite instance.

use std::process::exit;
use std::sync::mpsc::Receiver;
use std::thread;
use tracing::level_filters::LevelFilter;
use tracing::{error, warn};
use tracing_subscriber::EnvFilter;

use crate::decision::Decision;
use crate::network::Network;

mod common;
mod context;
mod decision;
mod network;

fn main() {
    // Some sensible defaults to make logging work
    init();

    // Create a network of 4 peers
    let (mut n, mut states, rx) = Network::new(4);

    // Spawn a thread in the background that handles decided values
    handle_decisions_background(rx);

    // Blocking method, starts the network and handles orchestration of
    // block building
    n.run(&mut states);

    // Todo: Clean stop
}

fn handle_decisions_background(rx: Receiver<Decision>) {
    thread::spawn(move || {
        // Busy loop, simply consume the decided heights
        loop {
            let res = rx.recv();
            match res {
                Ok(d) => {
                    warn!(
                        peer = %d.peer.to_string(),
                        value = %d.value.0.to_string(),
                        height = %d.height,
                        "new decision took place",
                    );
                }
                Err(err) => {
                    error!(error = ?err, "error receiving decisions");
                    error!("stopping");
                    exit(0);
                }
            }
        }
    });
}

fn init() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .from_env()
        .unwrap()
        .add_directive("loopback=info".parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .with_target(false)
        .init();
}
