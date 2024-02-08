#[path = "driver/runner.rs"]
pub mod runner;
#[path = "driver/utils.rs"]
pub mod utils;

use glob::glob;
use malachite_itf::driver::N4F1State;
use rand::rngs::StdRng;
use rand::SeedableRng;

// use malachite_itf::driver::State;
use malachite_itf::utils::{generate_traces, get_seed, TraceOptions};

// use runner::DriverRunner;

#[test]
fn test_itf() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_owned();

    if std::env::var("KEEP_TEMP").is_ok() {
        std::mem::forget(temp_dir);
    }

    let seed = get_seed();

    // generate_traces(
    //     "tests/line28/line28Test.qnt",
    //     &temp_path.to_string_lossy(),
    //     TraceOptions {
    //         seed,
    //         max_samples: 1,
    //     },
    // );

    let json = std::fs::read_to_string("/tmp/driver-mbt/outline28Test_0.itf.json").unwrap();
    let trace = itf::trace_from_str::<N4F1State>(&json).unwrap();
    dbg!(trace);

    // for json_fixture in glob(&format!("{}/*.itf.json", temp_path.display()))
    //     .expect("Failed to read glob pattern")
    //     .flatten()
    // {
    //     println!(
    //         "🚀 Running trace {:?}",
    //         json_fixture.file_name().unwrap().to_str().unwrap()
    //     );
    //
    //     let json = std::fs::read_to_string(&json_fixture).unwrap();
    //     let trace = itf::trace_from_str::<State>(&json).unwrap();
    //     dbg!(trace);
    //
    //     let mut rng = StdRng::seed_from_u64(seed);
    //
    //     // Build mapping from model addresses to real addresses
    //     let address_map = utils::build_address_map(&trace, &mut rng);
    //
    //     let runner = DriverRunner { address_map };
    //     trace.run_on(runner).unwrap();
    // }
}
