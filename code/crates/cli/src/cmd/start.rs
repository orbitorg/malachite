use clap::Parser;
use color_eyre::eyre::Result;
use tracing::{info, trace};

use malachite_node::config::{App, Config};
use malachite_test::{Address, PrivateKey, ValidatorSet};

use malachite_starknet_app::spawn::spawn_node_actor;

use crate::metrics;

#[derive(Parser, Debug, Clone, Default, PartialEq)]
pub struct StartCmd;

impl StartCmd {
    pub async fn run(&self, sk: PrivateKey, cfg: Config, vs: ValidatorSet) -> Result<()> {
        let val_address = Address::from_public_key(&sk.public_key());

        info!(
            validator_address = %val_address,
            "Found validator address."
        );

        if cfg.metrics.enabled {
            tokio::spawn(metrics::serve(cfg.metrics.clone()));
        }

        let (actor, handle) = match cfg.app {
            App::Starknet => spawn_node_actor(cfg, vs, sk.clone(), sk, val_address, None).await,
        };

        tokio::spawn({
            let actor = actor.clone();
            {
                async move {
                    tokio::signal::ctrl_c().await.unwrap();
                    info!("Termination signal received.");
                    actor.stop(None);
                }
            }
        });

        handle.await?;

        trace!("Node stopped.");

        Ok(())
    }
}
