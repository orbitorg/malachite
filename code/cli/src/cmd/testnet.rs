//! Testnet command

use std::path::Path;

use color_eyre::eyre::Result;
use rand::prelude::StdRng;
use rand::rngs::OsRng;
use rand::{Rng, SeedableRng};
use tracing::info;

use malachite_node::config::{Config, ConsensusConfig, MempoolConfig, P2pConfig, TimeoutConfig};
use malachite_test::ValidatorSet as Genesis;
use malachite_test::{PrivateKey, PublicKey, Validator};

use crate::args::Args;
use crate::cmd::init::{save_config, save_genesis, save_priv_validator_key};
use crate::priv_key::PrivValidatorKey;

const MIN_VOTING_POWER: u64 = 8;
const MAX_VOTING_POWER: u64 = 15;

/// Execute the testnet command
pub fn run(home_dir: &Path, nodes: usize, deterministic: bool) -> Result<()> {
    let private_keys = generate_private_keys(nodes, deterministic);
    let public_keys = private_keys.iter().map(|pk| pk.public_key()).collect();
    let genesis = generate_genesis(public_keys, deterministic);

    for (i, private_key) in private_keys.iter().enumerate().take(nodes) {
        // Use home directory `home_dir/<index>`
        let node_home_dir = home_dir.join(i.to_string());

        info!(
            "Generating configuration for node {i} at `{}`...",
            node_home_dir.display()
        );

        // Set the destination folder
        let args = Args {
            home: Some(node_home_dir),
            ..Args::default()
        };

        // Save private key
        let priv_validator_key = PrivValidatorKey::from(private_key.clone());
        save_priv_validator_key(
            &args.get_priv_validator_key_file_path()?,
            &priv_validator_key,
        )?;

        // Save genesis
        save_genesis(&args.get_genesis_file_path()?, &genesis)?;

        // Save config
        save_config(&args.get_config_file_path()?, &generate_config(i, nodes))?;
    }
    Ok(())
}

/// Generate private keys. Random or deterministic for different use-cases.
pub fn generate_private_keys(size: usize, deterministic: bool) -> Vec<PrivateKey> {
    if deterministic {
        let mut rng = StdRng::seed_from_u64(0x42);
        (0..size).map(|_| PrivateKey::generate(&mut rng)).collect()
    } else {
        (0..size).map(|_| PrivateKey::generate(OsRng)).collect()
    }
}

/// Generate a Genesis file from the public keys and voting power.
/// Voting power can be random or deterministically pseudo-random.
pub fn generate_genesis(pks: Vec<PublicKey>, deterministic: bool) -> Genesis {
    let size = pks.len();
    let voting_powers: Vec<u64> = if deterministic {
        let mut rng = StdRng::seed_from_u64(0x42);
        (0..size)
            .map(|_| rng.gen_range(MIN_VOTING_POWER..=MAX_VOTING_POWER))
            .collect()
    } else {
        (0..size)
            .map(|_| OsRng.gen_range(MIN_VOTING_POWER..=MAX_VOTING_POWER))
            .collect()
    };

    let mut validators = Vec::with_capacity(size);

    for i in 0..size {
        validators.push(Validator::new(pks[i], voting_powers[i]));
    }

    Genesis { validators }
}

const CONSENSUS_BASE_PORT: usize = 27000;
const MEMPOOL_BASE_PORT: usize = 28000;

/// Generate configuration for node "index" out of "total" number of nodes.
pub fn generate_config(index: usize, total: usize) -> Config {
    let consensus_port = CONSENSUS_BASE_PORT + index;
    let mempool_port = MEMPOOL_BASE_PORT + index;

    Config {
        moniker: format!("test-{}", index),
        consensus: ConsensusConfig {
            timeouts: TimeoutConfig::default(),
            p2p: P2pConfig {
                listen_addr: format!("/ip4/127.0.0.1/udp/{consensus_port}/quic-v1")
                    .parse()
                    .unwrap(),
                persistent_peers: (0..total)
                    .filter(|j| *j != index)
                    .map(|j| {
                        format!("/ip4/127.0.0.1/udp/{}/quic-v1", CONSENSUS_BASE_PORT + j)
                            .parse()
                            .unwrap()
                    })
                    .collect(),
            },
        },
        mempool: MempoolConfig {
            p2p: P2pConfig {
                listen_addr: format!("/ip4/127.0.0.1/udp/{mempool_port}/quic-v1")
                    .parse()
                    .unwrap(),
                persistent_peers: (0..total)
                    .filter(|j| *j != index)
                    .map(|j| {
                        format!("/ip4/127.0.0.1/udp/{}/quic-v1", MEMPOOL_BASE_PORT + j)
                            .parse()
                            .unwrap()
                    })
                    .collect(),
            },
        },
        test: Default::default(),
    }
}