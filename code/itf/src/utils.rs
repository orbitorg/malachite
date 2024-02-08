use glob::glob;
use std::path::Path;

#[derive(Copy, Clone, Debug)]
pub struct TraceOptions {
    pub seed: u64,
    pub max_samples: u64,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            seed: 42,
            max_samples: 100,
        }
    }
}

pub fn generate_traces(spec_rel_path: &str, gen_dir: &str, options: TraceOptions) {
    println!("🪄 Generating traces for {spec_rel_path:?}...");

    let spec_abs_path = format!(
        "{}/../../specs/quint/{}",
        env!("CARGO_MANIFEST_DIR"),
        spec_rel_path
    );

    let spec_path = Path::new(&spec_abs_path);

    std::process::Command::new("quint")
        .arg("test")
        .arg("--output")
        .arg(format!("{}/{{}}.itf.json", gen_dir))
        .arg("--seed")
        .arg(options.seed.to_string())
        .arg("--max-samples")
        .arg(options.max_samples.to_string())
        .arg(spec_path)
        .current_dir(spec_path.parent().unwrap())
        .output()
        .expect("Failed to run quint test");

    // Remove traces from imported modules
    for redundant_itf in glob(&format!(
        "{}/*{}::*.*",
        gen_dir,
        spec_path.file_stem().unwrap().to_str().unwrap()
    ))
    .expect("Failed to read glob pattern")
    .flatten()
    {
        std::fs::remove_file(&redundant_itf).unwrap();
    }

    println!("🪄 Generated traces in {gen_dir:?}");
}

pub fn get_seed() -> u64 {
    option_env!("QUINT_SEED")
        .map(|seed| {
            println!("using QUINT_SEED={seed}");
            seed
        })
        .or(Some("118"))
        .and_then(|x| x.parse::<u64>().ok())
        .filter(|&x| x != 0)
        .expect("invalid random seed for quint")
}
