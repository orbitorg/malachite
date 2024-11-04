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

use crate::context::BaseContext;
use crate::network::Network;

mod context;
mod network;

fn main() {
    // Create a network of 4 peers
    let n = Network::<BaseContext>::new(4);

    // Channels on which we'll receive the decided heights
    let (tx, rx) = mpsc::channel();

    // Start the network, handling orchestration of messages to build
    // blocks
    n.start(tx);

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
        // Todo: Clean stop
    }
}
