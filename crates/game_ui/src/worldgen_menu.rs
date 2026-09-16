use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use game_worldgen::{generate, image_export, nations, preset, stats};

use crate::main_menu::MainMenuScreen;
use crate::widgets::{BACKGROUND, NORMAL_BUTTON, button_node, stepper_button_node};

const PREVIEW_WIDTH: f32 = 560.0;
const PREVIEW_HEIGHT: f32 = 280.0;

const CONTINENTS_RANGE: (u32, u32) = (1, 60);
const NATIONS_RANGE: (u32, u32) = (1, 32);
const SEA_LEVEL_RANGE: (f32, f32) = (0.30, 0.70);
const BIAS_RANGE: (f32, f32) = (-0.30, 0.30);
/// Resolution stepper range (map width in pixels; height is always half —
/// see `WorldGenSettings::dimensions`). This changes image detail only, not
/// how much "world" there is — that's what Continents/Nations/etc. control.
/// 2048x1024 generates in ~1.75s (measured via the CLI); 3072x1536 takes
/// ~4.7s, noticeable enough as a per-click UI freeze that 2048 is the cap.
const RESOLUTION_RANGE: (u32, u32) = (128, 2048);
const RESOLUTION_STEP: u32 = 128;

const ZOOM_RANGE: (f32, f32) = (1.0, 6.0);
const ZOOM_STEP: f32 = 0.5;
/// Pan step in on-screen pixels per click of the pan pad — independent of
/// zoom level, so panning feels the same speed regardless of how zoomed in
/// the preview currently is.
const PAN_STEP: f32 = 48.0;

/// Preview pan/zoom state — a transform on top of the already-generated
/// preview image, not something that triggers regeneration. Kept as its own
/// resource (rather than folded into `WorldGenSettings`) so button_actions
/// can tell "the world needs regenerating" apart from "just the view
/// changed" at a glance.
#[derive(Resource, Clone, Copy)]
struct PreviewZoom {
    zoom: f32,
    pan: Vec2,
}

impl Default for PreviewZoom {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

impl PreviewZoom {
    /// Keeps `pan` in the range that leaves the (scaled) image fully
    /// covering the viewport — no gaps at the edges, and no panning past
    /// where there's nothing left to reveal. At `zoom == 1.0` this collapses
    /// both axes to exactly `0.0`, which is what makes zooming back out
    /// auto-recenter instead of leaving a stale offset.
    fn clamp_pan(&mut self) {
        let max_x = (PREVIEW_WIDTH * (self.zoom - 1.0)).max(0.0);
        let max_y = (PREVIEW_HEIGHT * (self.zoom - 1.0)).max(0.0);
        self.pan.x = self.pan.x.clamp(-max_x, 0.0);
        self.pan.y = self.pan.y.clamp(-max_y, 0.0);
    }
}

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
    view: PreviewView,
    /// When on, the preview shows `image_export::highlight_landmasses`
    /// instead of `view` — each distinct landmass in its own flat color, so
    /// the actual count and how cleanly separated they are is obvious at a
    /// glance, independent of biome coloring.
    highlight: bool,
    /// Whether `elevation::correct_continent_count` (merging/splitting
    /// landmasses to hit `continent_count`) runs at all — off shows the raw
    /// noise-generated landmasses untouched. See `Preset::correct_continents`.
    correct_continents: bool,
    /// Measured landmass count from the last generation — see
    /// `game_worldgen::stats::count_landmasses`. Distinct from
    /// `continent_count` (the input target): sea level, coastline
    /// fragmentation and randomness mean the two often don't match exactly.
    actual_continents: usize,
}

/// Which rendering of the generated world the preview shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewView {
    Biome,
    Elevation,
}

impl PreviewView {
    fn label(self) -> &'static str {
        match self {
            PreviewView::Biome => "Biome",
            PreviewView::Elevation => "Elevation",
        }
    }

    fn toggled(self) -> Self {
        match self {
            PreviewView::Biome => PreviewView::Elevation,
            PreviewView::Elevation => PreviewView::Biome,
        }
    }
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
            view: PreviewView::Biome,
            highlight: false,
            correct_continents: true,
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
        self.continent_count = p
            .continent_count
            .clamp(CONTINENTS_RANGE.0, CONTINENTS_RANGE.1);
        self.sea_level = p.sea_level.clamp(SEA_LEVEL_RANGE.0, SEA_LEVEL_RANGE.1);
        self.humidity = p.moisture_bias.clamp(BIAS_RANGE.0, BIAS_RANGE.1);
        self.temperature = p.temperature_bias.clamp(BIAS_RANGE.0, BIAS_RANGE.1);
    }

    /// Builds the actual `Preset` this generation run uses: the chosen
    /// preset's "flavor" (mountains, rivers, octaves) with this screen's
    /// directly-controlled knobs layered on top.
    fn effective_preset(&self) -> preset::Preset {
        let mut effective = preset::ALL[self.preset_index];
        effective.continent_count = self.continent_count;
        effective.sea_level = self.sea_level;
        effective.moisture_bias = self.humidity;
        effective.temperature_bias = self.temperature;
        effective.correct_continents = self.correct_continents;
        effective
    }
}

#[derive(Component)]
struct WorldGenRoot;

/// The absolutely-positioned, actual-size-times-zoom image node inside the
/// fixed-size, clipped `PreviewViewport` — see `PreviewZoom`.
#[derive(Component)]
struct PreviewImage;

/// The zoom stepper's value label ("x1.0") — kept separate from the generic
/// `ValueLabel(LabelKind)` mechanism since it reflects `PreviewZoom`, not
/// `WorldGenSettings`.
#[derive(Component)]
struct ZoomLabel;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    View,
    Highlight,
    SplitMerge,
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
    ToggleView,
    ToggleHighlight,
    ToggleCorrectContinents,
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
    ZoomOut,
    ZoomIn,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    ResetView,
    Back,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<WorldGenSettings>()
        .init_resource::<PreviewZoom>()
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
    zoom: Res<PreviewZoom>,
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
            spawn_preview_column(parent, handle, &zoom);
            spawn_legend_column(parent);
        });
}

/// The preview image, clipped to a fixed-size viewport with a zoom/pan
/// transform on top (see `PreviewZoom`), plus the zoom stepper and pan pad
/// that drive it.
fn spawn_preview_column(
    parent: &mut ChildSpawnerCommands,
    image: Handle<Image>,
    zoom: &PreviewZoom,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|column| {
            column
                .spawn(Node {
                    width: Val::Px(PREVIEW_WIDTH),
                    height: Val::Px(PREVIEW_HEIGHT),
                    overflow: Overflow::clip(),
                    position_type: PositionType::Relative,
                    ..default()
                })
                .with_children(|viewport| {
                    viewport.spawn((
                        PreviewImage,
                        Node {
                            width: Val::Px(PREVIEW_WIDTH * zoom.zoom),
                            height: Val::Px(PREVIEW_HEIGHT * zoom.zoom),
                            position_type: PositionType::Absolute,
                            left: Val::Px(zoom.pan.x),
                            top: Val::Px(zoom.pan.y),
                            ..default()
                        },
                        ImageNode::new(image),
                    ));
                });

            column
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Button,
                        WorldGenButton::ZoomOut,
                        stepper_button_node(),
                        BackgroundColor(NORMAL_BUTTON),
                    ))
                    .with_child((Text::new("-"), TextFont::from_font_size(18.0)));
                    row.spawn((
                        Text::new(zoom_text(zoom)),
                        ZoomLabel,
                        TextFont::from_font_size(16.0),
                        Node {
                            width: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                    ));
                    row.spawn((
                        Button,
                        WorldGenButton::ZoomIn,
                        stepper_button_node(),
                        BackgroundColor(NORMAL_BUTTON),
                    ))
                    .with_child((Text::new("+"), TextFont::from_font_size(18.0)));

                    for (label, action) in [
                        ("<", WorldGenButton::PanLeft),
                        ("^", WorldGenButton::PanUp),
                        ("v", WorldGenButton::PanDown),
                        (">", WorldGenButton::PanRight),
                    ] {
                        row.spawn((
                            Button,
                            action,
                            stepper_button_node(),
                            BackgroundColor(NORMAL_BUTTON),
                        ))
                        .with_child((Text::new(label), TextFont::from_font_size(16.0)));
                    }

                    row.spawn((
                        Button,
                        WorldGenButton::ResetView,
                        Node {
                            width: Val::Px(96.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(NORMAL_BUTTON),
                    ))
                    .with_child((Text::new("Reset View"), TextFont::from_font_size(13.0)));
                });
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
                "View",
                LabelKind::View,
                settings.view.label().to_string(),
                WorldGenButton::ToggleView,
                WorldGenButton::ToggleView,
            );
            spawn_row(
                column,
                "Highlight",
                LabelKind::Highlight,
                highlight_text(settings),
                WorldGenButton::ToggleHighlight,
                WorldGenButton::ToggleHighlight,
            );
            spawn_row(
                column,
                "Split/Merge",
                LabelKind::SplitMerge,
                split_merge_text(settings),
                WorldGenButton::ToggleCorrectContinents,
                WorldGenButton::ToggleCorrectContinents,
            );
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

fn highlight_text(settings: &WorldGenSettings) -> String {
    if settings.highlight { "On" } else { "Off" }.to_string()
}

fn split_merge_text(settings: &WorldGenSettings) -> String {
    if settings.correct_continents {
        "On"
    } else {
        "Off"
    }
    .to_string()
}

fn zoom_text(zoom: &PreviewZoom) -> String {
    format!("x{:.1}", zoom.zoom)
}

#[allow(clippy::too_many_arguments)]
fn button_actions(
    buttons: Query<(&Interaction, &WorldGenButton), Changed<Interaction>>,
    mut settings: ResMut<WorldGenSettings>,
    mut zoom: ResMut<PreviewZoom>,
    mut labels: Query<(&mut Text, &ValueLabel), Without<ZoomLabel>>,
    mut zoom_label: Query<&mut Text, (With<ZoomLabel>, Without<ValueLabel>)>,
    mut images: ResMut<Assets<Image>>,
    mut preview: Query<(&mut ImageNode, &mut Node), With<PreviewImage>>,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
) {
    let mut changed = false;
    let mut view_changed = false;

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            WorldGenButton::ToggleView => {
                settings.view = settings.view.toggled();
                changed = true;
            }
            WorldGenButton::ToggleHighlight => {
                settings.highlight = !settings.highlight;
                changed = true;
            }
            WorldGenButton::ToggleCorrectContinents => {
                settings.correct_continents = !settings.correct_continents;
                changed = true;
            }
            WorldGenButton::ZoomOut => {
                zoom.zoom = (zoom.zoom - ZOOM_STEP).max(ZOOM_RANGE.0);
                zoom.clamp_pan();
                view_changed = true;
            }
            WorldGenButton::ZoomIn => {
                zoom.zoom = (zoom.zoom + ZOOM_STEP).min(ZOOM_RANGE.1);
                zoom.clamp_pan();
                view_changed = true;
            }
            WorldGenButton::PanLeft => {
                zoom.pan.x += PAN_STEP;
                zoom.clamp_pan();
                view_changed = true;
            }
            WorldGenButton::PanRight => {
                zoom.pan.x -= PAN_STEP;
                zoom.clamp_pan();
                view_changed = true;
            }
            WorldGenButton::PanUp => {
                zoom.pan.y += PAN_STEP;
                zoom.clamp_pan();
                view_changed = true;
            }
            WorldGenButton::PanDown => {
                zoom.pan.y -= PAN_STEP;
                zoom.clamp_pan();
                view_changed = true;
            }
            WorldGenButton::ResetView => {
                *zoom = PreviewZoom::default();
                view_changed = true;
            }
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

    if view_changed {
        if let Ok(mut text) = zoom_label.single_mut() {
            text.0 = zoom_text(&zoom);
        }
        if let Ok((_, mut node)) = preview.single_mut() {
            node.width = Val::Px(PREVIEW_WIDTH * zoom.zoom);
            node.height = Val::Px(PREVIEW_HEIGHT * zoom.zoom);
            node.left = Val::Px(zoom.pan.x);
            node.top = Val::Px(zoom.pan.y);
        }
    }

    if !changed {
        return;
    }

    if let Ok((mut image_node, _)) = preview.single_mut() {
        image_node.image = images.add(generate_image(&mut settings));
    }

    for (mut text, label) in &mut labels {
        text.0 = label_text(&settings, label.0);
    }
}

fn label_text(settings: &WorldGenSettings, kind: LabelKind) -> String {
    match kind {
        LabelKind::View => settings.view.label().to_string(),
        LabelKind::Highlight => highlight_text(settings),
        LabelKind::SplitMerge => split_merge_text(settings),
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
/// `WorldGenSettings::actual_continents`), and returns the rendered preview
/// for whichever view is currently selected — the world itself is generated
/// once either way, only the final rendering differs.
fn generate_image(settings: &mut WorldGenSettings) -> Image {
    let effective_preset = settings.effective_preset();
    let (width, height) = settings.dimensions();

    let world = generate(width, height, settings.seed, &effective_preset);

    // Ignore specks smaller than ~0.15% of the map — otherwise single-pixel
    // noise artifacts would inflate the "actual" count past anything a
    // player would call a continent.
    let min_landmass_size = ((width * height) as f64 * 0.0015).max(1.0) as usize;
    let (landmass_labels, actual_continents) =
        stats::label_landmasses(&world.biome.terrain, min_landmass_size);
    settings.actual_continents = actual_continents;

    let rgb_image = if settings.highlight {
        image_export::highlight_landmasses(width, height, &landmass_labels)
    } else {
        match settings.view {
            PreviewView::Biome => {
                let nation_positions =
                    nations::place(&world, settings.nation_count as usize, settings.seed);
                let mut image = image_export::biome_map(
                    width,
                    height,
                    &world.elevation,
                    &world.hydrology,
                    &world.biome,
                );
                image_export::draw_nations(&mut image, &nation_positions);
                image
            }
            PreviewView::Elevation => image_export::elevation_hypsometric(
                width,
                height,
                &world.elevation,
                effective_preset.sea_level,
            ),
        }
    };

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
