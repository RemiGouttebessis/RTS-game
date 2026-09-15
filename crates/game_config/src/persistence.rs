use std::fs;
use std::path::PathBuf;

use crate::Settings;

const FILE_NAME: &str = "settings.ron";

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "rts-game").map(|dirs| dirs.config_dir().join(FILE_NAME))
}

/// Loads settings from disk, falling back to defaults if there's no config
/// directory, no file yet, or the file fails to parse. A file that fails to
/// parse is left untouched (only reported) so a hand-edit mistake doesn't get
/// silently overwritten; a missing file is created with defaults so there's
/// always something on disk to edit.
pub fn load() -> Settings {
    let Some(path) = config_path() else {
        eprintln!("Could not determine a config directory; using default settings.");
        return Settings::default();
    };

    match fs::read_to_string(&path) {
        Ok(contents) => ron::from_str(&contents).unwrap_or_else(|err| {
            eprintln!(
                "Failed to parse settings at {}: {err}. Using defaults for this run.",
                path.display()
            );
            Settings::default()
        }),
        Err(_) => {
            let settings = Settings::default();
            save(&settings);
            settings
        }
    }
}

/// Persists settings to disk so other launches of the program pick them up.
pub fn save(settings: &Settings) {
    let Some(path) = config_path() else {
        eprintln!("Could not determine a config directory; settings were not saved.");
        return;
    };

    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        eprintln!(
            "Failed to create config directory {}: {err}",
            parent.display()
        );
        return;
    }

    let contents = match ron::ser::to_string_pretty(settings, ron::ser::PrettyConfig::default()) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("Failed to serialize settings: {err}");
            return;
        }
    };

    if let Err(err) = fs::write(&path, contents) {
        eprintln!("Failed to write settings to {}: {err}", path.display());
    }
}
