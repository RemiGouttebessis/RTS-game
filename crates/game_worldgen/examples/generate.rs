//! Standalone world generator preview.
//!
//! ```text
//! cargo run -p game_worldgen --example generate -- \
//!     --preset continents --seed 42 --width 512 --height 256 --out world.png
//! ```
//!
//! Also writes `<out>.elevation.png` (raw grayscale heightmap) alongside the
//! biome map, useful for sanity-checking generation parameters.

use std::path::PathBuf;

use game_worldgen::{generate, image_export, preset};

struct Args {
    preset: String,
    seed: u64,
    width: usize,
    height: usize,
    out: PathBuf,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            preset: "continents".to_string(),
            seed: 1,
            width: 512,
            height: 256,
            out: PathBuf::from("world.png"),
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut raw = std::env::args().skip(1);

    while let Some(flag) = raw.next() {
        let mut value = || raw.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--preset" => args.preset = value()?,
            "--seed" => args.seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--width" => args.width = value()?.parse().map_err(|e| format!("--width: {e}"))?,
            "--height" => args.height = value()?.parse().map_err(|e| format!("--height: {e}"))?,
            "--out" => args.out = PathBuf::from(value()?),
            "--list-presets" => {
                for p in preset::ALL {
                    println!("{}", p.name);
                }
                std::process::exit(0);
            }
            other => return Err(format!("unknown flag '{other}'")),
        }
    }

    Ok(args)
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "usage: generate --preset <name> --seed <u64> --width <n> --height <n> --out <path>"
            );
            eprintln!("       generate --list-presets");
            std::process::exit(1);
        }
    };

    let Some(chosen_preset) = preset::by_name(&args.preset) else {
        eprintln!(
            "unknown preset '{}'; available: {}",
            args.preset,
            preset::ALL
                .iter()
                .map(|p| p.name)
                .collect::<Vec<_>>()
                .join(", ")
        );
        std::process::exit(1);
    };

    println!(
        "generating {}x{} '{}' world, seed {}...",
        args.width, args.height, chosen_preset.name, args.seed
    );

    let world = generate(args.width, args.height, args.seed, &chosen_preset);

    let biome_image = image_export::biome_map(
        args.width,
        args.height,
        &world.elevation,
        &world.hydrology,
        &world.biome,
    );
    biome_image
        .save(&args.out)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", args.out.display()));
    println!("wrote {}", args.out.display());

    let elevation_path = args.out.with_extension("elevation.png");
    let elevation_image =
        image_export::elevation_grayscale(args.width, args.height, &world.elevation);
    elevation_image
        .save(&elevation_path)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", elevation_path.display()));
    println!("wrote {}", elevation_path.display());
}
