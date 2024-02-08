#[path = "consensus/runner.rs"]
pub mod runner;
#[path = "consensus/utils.rs"]
pub mod utils;

use glob::glob;
use rand::rngs::StdRng;
use rand::SeedableRng;

use malachite_itf::consensus::State;
use malachite_itf::utils::{generate_traces, get_seed, TraceOptions};

use runner::ConsensusRunner;

#[test]
fn test_itf() {
    let temp_dir =
        tempfile::TempDir::with_prefix("malachite-consensus-").expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_owned();

    if std::env::var("KEEP_TEMP").is_ok() {
        std::mem::forget(temp_dir);
    }

    let seed = get_seed();

    generate_traces(
        "tests/consensus/consensusTest.qnt",
        &temp_path.to_string_lossy(),
        TraceOptions {
            seed,
            ..Default::default()
        },
    );

    for json_fixture in glob(&format!("{}/*.itf.json", temp_path.display()))
        .expect("Failed to read glob pattern")
        .flatten()
    {
        println!(
            "🚀 Running trace {:?}",
            json_fixture.file_name().unwrap().to_str().unwrap()
        );

        let json = std::fs::read_to_string(&json_fixture).unwrap();
        let trace = itf::trace_from_str::<State>(&json).unwrap();

        let mut rng = StdRng::seed_from_u64(seed);

        // Build mapping from model addresses to real addresses
        let address_map = utils::build_address_map(&trace, &mut rng);

        let consensus_runner = ConsensusRunner { address_map };

        trace.run_on(consensus_runner).unwrap();
    }
}
