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
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

use crate::context::height::BaseHeight;
use crate::network::Network;

mod context;
mod network;

fn main() {
    // Create a network of 4 peers
    let (mut n, mut states) = Network::new(4);

    // Channels on which we'll receive the decided heights
    let (tx, rx) = mpsc::channel();

    // Spawn a thread in the background that handles decided values
    handle_decisions_background(rx);

    // Blocking method, starts the network and handles orchestration of
    // block building
    n.run(tx, &mut states);

    // Todo: Clean stop
}

fn handle_decisions_background(rx: Receiver<BaseHeight>) {
    thread::spawn(move || {
        // Busy loop, simply consume the decided heights
        loop {
            let res = rx.recv();
            match res {
                Ok(height) => {
                    println!("new height decided: {}", height);
                }
                Err(err) => {
                    println!("unable to decide on new height with message: {:?}", err);
                    println!("stopping");
                    exit(0);
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });
}
