use std::path::PathBuf;

use color_eyre::eyre::Result;
use malachite_test::{Address, PrivateKey};
use tracing::info;

use crate::args::Args;

#[derive(clap::Args, Clone, Debug)]
pub struct GenerateCmd {
    #[clap(short, long, value_name = "OUTPUT_FILE")]
    output: PathBuf,
}

impl GenerateCmd {
    pub fn run(&self, _args: &Args) -> Result<()> {
        let rng = rand::thread_rng();
        let pk = PrivateKey::generate(rng);

        let address = Address::from_public_key(&pk.public_key());

        let public_key = pk.public_key();

        info!(validator_address = %address, pub_key=serde_json::to_string(&public_key)?, "Generated key.");

        info!(file=%self.output.display(), "Saving private key.");
        std::fs::write(&self.output, serde_json::to_vec(&pk)?)?;

        Ok(())
    }
}
