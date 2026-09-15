use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use game_worldgen::{generate, image_export, nations, preset, stats};

use crate::main_menu::MainMenuScreen;
use crate::widgets::{BACKGROUND, NORMAL_BUTTON, button_node, stepper_button_node};

const PREVIEW_WIDTH: f32 = 560.0;
const PREVIEW_HEIGHT: f32 = 280.0;

const CONTINENTS_RANGE: (u32, u32) = (1, 20);
const NATIONS_RANGE: (u32, u32) = (1, 32);
const SEA_LEVEL_RANGE: (f32, f32) = (0.30, 0.70);
const BIAS_RANGE: (f32, f32) = (-0.30, 0.30);
/// Resolution stepper range (map width in pixels; height is always half —
/// see `WorldGenSettings::dimensions`). This changes image detail only, not
/// how much "world" there is — that's what Continents/Nations/etc. control.
const RESOLUTION_RANGE: (u32, u32) = (128, 1024);
const RESOLUTION_STEP: u32 = 64;

/// Current picks on the world-gen screen. Survives leaving/re-entering the
/// screen (it's a normal resource, not reset by `OnEnter`), so navigating
/// Back and returning to World Gen doesn't lose your settings.
#[derive(Resource, Clone, Copy)]
struct WorldGenSettings {
    preset_index: usize,
    resolution: u32,
    continent_count: u32,
    sea_level: f32,
    humidity: f32,
    temperature: f32,
    nation_count: u32,
    seed: u64,
    /// Measured landmass count from the last generation — see
    /// `game_worldgen::stats::count_landmasses`. Distinct from
    /// `continent_count` (the input target): sea level, coastline
    /// fragmentation and randomness mean the two often don't match exactly.
    actual_continents: usize,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        let preset_index = 0;
        let mut settings = Self {
            preset_index,
            resolution: 512,
            continent_count: 4,
            sea_level: 0.5,
            humidity: 0.0,
            temperature: 0.0,
            nation_count: 8,
            seed: 1,
            actual_continents: 0,
        };
        settings.apply_preset_defaults();
        settings
    }
}

impl WorldGenSettings {
    fn dimensions(&self) -> (usize, usize) {
        (self.resolution as usize, (self.resolution / 2) as usize)
    }

    /// Loads the selected preset's own values into the individually-tunable
    /// fields. Called whenever the preset changes, so e.g. picking
    /// "archipelago" actually produces an archipelago instead of silently
    /// keeping whatever continent count/sea level was left over from
    /// whichever preset (or default) was selected before.
    fn apply_preset_defaults(&mut self) {
        let p = preset::ALL[self.preset_index];
        self.continent_count = (preset::count_for_radius(p.continent_radius).round() as u32)
            .clamp(CONTINENTS_RANGE.0, CONTINENTS_RANGE.1);
        self.sea_level = p.sea_level.clamp(SEA_LEVEL_RANGE.0, SEA_LEVEL_RANGE.1);
        self.humidity = p.moisture_bias.clamp(BIAS_RANGE.0, BIAS_RANGE.1);
        self.temperature = p.temperature_bias.clamp(BIAS_RANGE.0, BIAS_RANGE.1);
    }

    /// Builds the actual `Preset` this generation run uses: the chosen
    /// preset's "flavor" (mountains, rivers, octaves) with this screen's
    /// four directly-controlled knobs layered on top.
    fn effective_preset(&self) -> preset::Preset {
        let mut effective = preset::ALL[self.preset_index];
        effective.continent_radius = preset::radius_for_count(self.continent_count as f64);
        effective.sea_level = self.sea_level;
        effective.moisture_bias = self.humidity;
        effective.temperature_bias = self.temperature;
        effective
    }
}

#[derive(Component)]
struct WorldGenRoot;

#[derive(Component)]
struct PreviewImage;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    Preset,
    Resolution,
    Continents,
    SeaLevel,
    Humidity,
    Temperature,
    Nations,
    Seed,
}

#[derive(Component)]
struct ValueLabel(LabelKind);

#[derive(Component, Clone, Copy)]
enum WorldGenButton {
    PresetPrev,
    PresetNext,
    ResolutionDec,
    ResolutionInc,
    ContinentsDec,
    ContinentsInc,
    SeaLevelDec,
    SeaLevelInc,
    HumidityDec,
    HumidityInc,
    TemperatureDec,
    TemperatureInc,
    NationsDec,
    NationsInc,
    SeedDec,
    SeedInc,
    RandomizeSeed,
    Back,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<WorldGenSettings>()
        .add_systems(OnEnter(MainMenuScreen::WorldGen), spawn_screen)
        .add_systems(OnExit(MainMenuScreen::WorldGen), despawn_screen)
        .add_systems(
            Update,
            button_actions.run_if(in_state(MainMenuScreen::WorldGen)),
        );
}

fn spawn_screen(
    mut commands: Commands,
    mut settings: ResMut<WorldGenSettings>,
    mut images: ResMut<Assets<Image>>,
) {
    let handle = images.add(generate_image(&mut settings));

    commands
        .spawn((
            WorldGenRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            spawn_settings_column(parent, &settings);

            parent.spawn((
                PreviewImage,
                Node {
                    width: Val::Px(PREVIEW_WIDTH),
                    height: Val::Px(PREVIEW_HEIGHT),
                    ..default()
                },
                ImageNode::new(handle),
            ));

            spawn_legend_column(parent);
        });
}

fn spawn_settings_column(parent: &mut ChildSpawnerCommands, settings: &WorldGenSettings) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|column| {
            column.spawn((Text::new("World Generator"), TextFont::from_font_size(28.0)));

            spawn_row(
                column,
                "Preset",
                LabelKind::Preset,
                preset::ALL[settings.preset_index].name.to_string(),
                WorldGenButton::PresetPrev,
                WorldGenButton::PresetNext,
            );
            spawn_row(
                column,
                "Resolution",
                LabelKind::Resolution,
                resolution_text(settings),
                WorldGenButton::ResolutionDec,
                WorldGenButton::ResolutionInc,
            );
            spawn_row(
                column,
                "Continents",
                LabelKind::Continents,
                continents_text(settings),
                WorldGenButton::ContinentsDec,
                WorldGenButton::ContinentsInc,
            );
            spawn_row(
                column,
                "Sea level",
                LabelKind::SeaLevel,
                format!("{:.2}", settings.sea_level),
                WorldGenButton::SeaLevelDec,
                WorldGenButton::SeaLevelInc,
            );
            spawn_row(
                column,
                "Humidity",
                LabelKind::Humidity,
                format!("{:+.2}", settings.humidity),
                WorldGenButton::HumidityDec,
                WorldGenButton::HumidityInc,
            );
            spawn_row(
                column,
                "Temperature",
                LabelKind::Temperature,
                format!("{:+.2}", settings.temperature),
                WorldGenButton::TemperatureDec,
                WorldGenButton::TemperatureInc,
            );
            spawn_row(
                column,
                "Nations",
                LabelKind::Nations,
                settings.nation_count.to_string(),
                WorldGenButton::NationsDec,
                WorldGenButton::NationsInc,
            );
            spawn_row(
                column,
                "Seed",
                LabelKind::Seed,
                settings.seed.to_string(),
                WorldGenButton::SeedDec,
                WorldGenButton::SeedInc,
            );

            column
                .spawn((
                    Button,
                    WorldGenButton::RandomizeSeed,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Randomize Seed"), TextFont::from_font_size(18.0)));

            column
                .spawn((
                    Button,
                    WorldGenButton::Back,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Back"), TextFont::from_font_size(18.0)));
        });
}

fn spawn_legend_column(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|column| {
            column.spawn((Text::new("Legend"), TextFont::from_font_size(20.0)));

            for (label, color) in image_export::legend() {
                column
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: Val::Px(16.0),
                                height: Val::Px(16.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb_u8(color[0], color[1], color[2])),
                            BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
                        ));
                        row.spawn((Text::new(label), TextFont::from_font_size(14.0)));
                    });
            }
        });
}

fn spawn_row(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    kind: LabelKind,
    initial_value: String,
    dec: WorldGenButton,
    inc: WorldGenButton,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont::from_font_size(16.0),
                Node {
                    width: Val::Px(110.0),
                    ..default()
                },
            ));
            row.spawn((
                Button,
                dec,
                stepper_button_node(),
                BackgroundColor(NORMAL_BUTTON),
            ))
            .with_child((Text::new("-"), TextFont::from_font_size(18.0)));
            row.spawn((
                Text::new(initial_value),
                ValueLabel(kind),
                TextFont::from_font_size(16.0),
                Node {
                    width: Val::Px(110.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ));
            row.spawn((
                Button,
                inc,
                stepper_button_node(),
                BackgroundColor(NORMAL_BUTTON),
            ))
            .with_child((Text::new("+"), TextFont::from_font_size(18.0)));
        });
}

fn resolution_text(settings: &WorldGenSettings) -> String {
    let (width, height) = settings.dimensions();
    format!("{width}x{height}")
}

fn continents_text(settings: &WorldGenSettings) -> String {
    format!(
        "{} (~{})",
        settings.continent_count, settings.actual_continents
    )
}

#[allow(clippy::too_many_arguments)]
fn button_actions(
    buttons: Query<(&Interaction, &WorldGenButton), Changed<Interaction>>,
    mut settings: ResMut<WorldGenSettings>,
    mut labels: Query<(&mut Text, &ValueLabel)>,
    mut images: ResMut<Assets<Image>>,
    mut preview: Query<&mut ImageNode, With<PreviewImage>>,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
) {
    let mut changed = false;

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            WorldGenButton::PresetPrev => {
                settings.preset_index =
                    (settings.preset_index + preset::ALL.len() - 1) % preset::ALL.len();
                settings.apply_preset_defaults();
                changed = true;
            }
            WorldGenButton::PresetNext => {
                settings.preset_index = (settings.preset_index + 1) % preset::ALL.len();
                settings.apply_preset_defaults();
                changed = true;
            }
            WorldGenButton::ResolutionDec => {
                settings.resolution =
                    (settings.resolution.saturating_sub(RESOLUTION_STEP)).max(RESOLUTION_RANGE.0);
                changed = true;
            }
            WorldGenButton::ResolutionInc => {
                settings.resolution =
                    (settings.resolution + RESOLUTION_STEP).min(RESOLUTION_RANGE.1);
                changed = true;
            }
            WorldGenButton::ContinentsDec => {
                settings.continent_count = settings
                    .continent_count
                    .saturating_sub(1)
                    .max(CONTINENTS_RANGE.0);
                changed = true;
            }
            WorldGenButton::ContinentsInc => {
                settings.continent_count = (settings.continent_count + 1).min(CONTINENTS_RANGE.1);
                changed = true;
            }
            WorldGenButton::SeaLevelDec => {
                settings.sea_level = (settings.sea_level - 0.02).max(SEA_LEVEL_RANGE.0);
                changed = true;
            }
            WorldGenButton::SeaLevelInc => {
                settings.sea_level = (settings.sea_level + 0.02).min(SEA_LEVEL_RANGE.1);
                changed = true;
            }
            WorldGenButton::HumidityDec => {
                settings.humidity = (settings.humidity - 0.05).max(BIAS_RANGE.0);
                changed = true;
            }
            WorldGenButton::HumidityInc => {
                settings.humidity = (settings.humidity + 0.05).min(BIAS_RANGE.1);
                changed = true;
            }
            WorldGenButton::TemperatureDec => {
                settings.temperature = (settings.temperature - 0.05).max(BIAS_RANGE.0);
                changed = true;
            }
            WorldGenButton::TemperatureInc => {
                settings.temperature = (settings.temperature + 0.05).min(BIAS_RANGE.1);
                changed = true;
            }
            WorldGenButton::NationsDec => {
                settings.nation_count =
                    settings.nation_count.saturating_sub(1).max(NATIONS_RANGE.0);
                changed = true;
            }
            WorldGenButton::NationsInc => {
                settings.nation_count = (settings.nation_count + 1).min(NATIONS_RANGE.1);
                changed = true;
            }
            WorldGenButton::SeedDec => {
                settings.seed = settings.seed.wrapping_sub(1);
                changed = true;
            }
            WorldGenButton::SeedInc => {
                settings.seed = settings.seed.wrapping_add(1);
                changed = true;
            }
            WorldGenButton::RandomizeSeed => {
                // A fixed-increment LCG step is enough here: we just need a
                // different-looking seed on each click, not real entropy.
                settings.seed = settings
                    .seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                changed = true;
            }
            WorldGenButton::Back => next_screen.set(MainMenuScreen::SoloMode),
        }
    }

    if !changed {
        return;
    }

    if let Ok(mut node) = preview.single_mut() {
        node.image = images.add(generate_image(&mut settings));
    }

    for (mut text, label) in &mut labels {
        text.0 = label_text(&settings, label.0);
    }
}

fn label_text(settings: &WorldGenSettings, kind: LabelKind) -> String {
    match kind {
        LabelKind::Preset => preset::ALL[settings.preset_index].name.to_string(),
        LabelKind::Resolution => resolution_text(settings),
        LabelKind::Continents => continents_text(settings),
        LabelKind::SeaLevel => format!("{:.2}", settings.sea_level),
        LabelKind::Humidity => format!("{:+.2}", settings.humidity),
        LabelKind::Temperature => format!("{:+.2}", settings.temperature),
        LabelKind::Nations => settings.nation_count.to_string(),
        LabelKind::Seed => settings.seed.to_string(),
    }
}

/// Generates the world, updates `settings.actual_continents` from it (see
/// `WorldGenSettings::actual_continents`), and returns the rendered preview.
fn generate_image(settings: &mut WorldGenSettings) -> Image {
    let effective_preset = settings.effective_preset();
    let (width, height) = settings.dimensions();

    let world = generate(width, height, settings.seed, &effective_preset);
    let nation_positions = nations::place(&world, settings.nation_count as usize, settings.seed);

    // Ignore specks smaller than ~0.15% of the map — otherwise single-pixel
    // noise artifacts would inflate the "actual" count past anything a
    // player would call a continent.
    let min_landmass_size = ((width * height) as f64 * 0.0015).max(1.0) as usize;
    settings.actual_continents = stats::count_landmasses(&world.biome.terrain, min_landmass_size);

    let mut rgb_image = image_export::biome_map(
        width,
        height,
        &world.elevation,
        &world.hydrology,
        &world.biome,
    );
    image_export::draw_nations(&mut rgb_image, &nation_positions);

    rgb_to_bevy_image(&rgb_image)
}

fn rgb_to_bevy_image(rgb: &image::RgbImage) -> Image {
    let mut data = Vec::with_capacity(rgb.width() as usize * rgb.height() as usize * 4);
    for pixel in rgb.pixels() {
        data.extend_from_slice(&pixel.0);
        data.push(255);
    }
    Image::new(
        Extent3d {
            width: rgb.width(),
            height: rgb.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn despawn_screen(mut commands: Commands, roots: Query<Entity, With<WorldGenRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
