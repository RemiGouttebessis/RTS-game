//! Standalone world generator preview.
//!
//! ```text
//! cargo run -p game_worldgen --example generate -- \
//!     --preset continents --seed 42 --width 512 --height 256 \
//!     --continents 4 --sea-level 0.5 --humidity 0.0 --temperature 0.0 \
//!     --nations 8 --out world.png
//! ```
//!
//! Also writes `<out>.elevation.png` (raw grayscale heightmap) alongside the
//! biome map, useful for sanity-checking generation parameters.
//!
//! `--continents`/`--sea-level`/`--humidity`/`--temperature` override the
//! chosen preset's values — same knobs `game_ui::worldgen_menu` exposes as
//! +/- steppers, kept in sync here for faster CLI iteration.

use std::path::PathBuf;

use game_worldgen::{generate, image_export, nations, preset, stats};

struct Args {
    preset: String,
    seed: u64,
    width: usize,
    height: usize,
    continents: Option<f64>,
    sea_level: Option<f32>,
    humidity: Option<f32>,
    temperature: Option<f32>,
    nation_count: usize,
    out: PathBuf,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            preset: "continents".to_string(),
            seed: 1,
            width: 512,
            height: 256,
            continents: None,
            sea_level: None,
            humidity: None,
            temperature: None,
            nation_count: 8,
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
            "--continents" => {
                args.continents = Some(value()?.parse().map_err(|e| format!("--continents: {e}"))?)
            }
            "--sea-level" => {
                args.sea_level = Some(value()?.parse().map_err(|e| format!("--sea-level: {e}"))?)
            }
            "--humidity" => {
                args.humidity = Some(value()?.parse().map_err(|e| format!("--humidity: {e}"))?)
            }
            "--temperature" => {
                args.temperature = Some(
                    value()?
                        .parse()
                        .map_err(|e| format!("--temperature: {e}"))?,
                )
            }
            "--nations" => {
                args.nation_count = value()?.parse().map_err(|e| format!("--nations: {e}"))?
            }
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
                "usage: generate --preset <name> --seed <u64> --width <n> --height <n> \
                 [--continents <n>] [--sea-level <0-1>] [--humidity <-1-1>] \
                 [--temperature <-1-1>] [--nations <n>] --out <path>"
            );
            eprintln!("       generate --list-presets");
            std::process::exit(1);
        }
    };

    let Some(mut chosen_preset) = preset::by_name(&args.preset) else {
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

    if let Some(continents) = args.continents {
        chosen_preset.continent_radius = preset::radius_for_count(continents);
    }
    if let Some(sea_level) = args.sea_level {
        chosen_preset.sea_level = sea_level;
    }
    if let Some(humidity) = args.humidity {
        chosen_preset.moisture_bias = humidity;
    }
    if let Some(temperature) = args.temperature {
        chosen_preset.temperature_bias = temperature;
    }

    println!(
        "generating {}x{} '{}' world, seed {}, {} nations...",
        args.width, args.height, chosen_preset.name, args.seed, args.nation_count
    );

    let world = generate(args.width, args.height, args.seed, &chosen_preset);
    let nation_positions = nations::place(&world, args.nation_count, args.seed);

    let min_landmass_size = ((args.width * args.height) as f64 * 0.0015).max(1.0) as usize;
    let actual_continents = stats::count_landmasses(&world.biome.terrain, min_landmass_size);
    println!("actual landmasses (>= {min_landmass_size} cells): {actual_continents}");

    let mut biome_image = image_export::biome_map(
        args.width,
        args.height,
        &world.elevation,
        &world.hydrology,
        &world.biome,
    );
    image_export::draw_nations(&mut biome_image, &nation_positions);
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
