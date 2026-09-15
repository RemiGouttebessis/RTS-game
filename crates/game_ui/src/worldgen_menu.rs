use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use game_worldgen::{generate, image_export, preset};

use crate::main_menu::MainMenuScreen;
use crate::widgets::{BACKGROUND, NORMAL_BUTTON, button_node, fullscreen_menu_node};

const PREVIEW_WIDTH: f32 = 480.0;
const PREVIEW_HEIGHT: f32 = 240.0;

/// Map dimensions offered in the UI — actual generation width/height, kept
/// small in number so they're cycle-able with a button instead of needing a
/// text input widget.
const MAP_SIZES: [(&str, usize, usize); 3] = [
    ("Small", 256, 128),
    ("Medium", 512, 256),
    ("Large", 768, 384),
];

/// Current picks on the world-gen screen. Survives leaving/re-entering the
/// screen (it's a normal resource, not reset by `OnEnter`), so navigating
/// Back and returning to World Gen doesn't lose your settings.
#[derive(Resource, Clone, Copy)]
struct WorldGenSettings {
    preset_index: usize,
    size_index: usize,
    seed: u64,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        Self {
            preset_index: 0,
            size_index: 1,
            seed: 1,
        }
    }
}

#[derive(Component)]
struct WorldGenRoot;

#[derive(Component)]
struct PresetLabel;

#[derive(Component)]
struct SizeLabel;

#[derive(Component)]
struct SeedLabel;

#[derive(Component)]
struct PreviewImage;

#[derive(Component, Clone, Copy)]
enum WorldGenButton {
    CyclePreset,
    CycleSize,
    RandomizeSeed,
    Generate,
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

fn spawn_screen(mut commands: Commands, settings: Res<WorldGenSettings>) {
    commands
        .spawn((
            WorldGenRoot,
            fullscreen_menu_node(),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("World Generator"), TextFont::from_font_size(36.0)));

            parent
                .spawn((
                    Button,
                    WorldGenButton::CyclePreset,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((
                    Text::new(format!(
                        "Preset: {}",
                        preset::ALL[settings.preset_index].name
                    )),
                    PresetLabel,
                    TextFont::from_font_size(20.0),
                ));

            parent
                .spawn((
                    Button,
                    WorldGenButton::CycleSize,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((
                    Text::new(format!("Size: {}", MAP_SIZES[settings.size_index].0)),
                    SizeLabel,
                    TextFont::from_font_size(20.0),
                ));

            parent
                .spawn((
                    Button,
                    WorldGenButton::RandomizeSeed,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((
                    Text::new(seed_label(settings.seed)),
                    SeedLabel,
                    TextFont::from_font_size(20.0),
                ));

            parent
                .spawn((
                    Button,
                    WorldGenButton::Generate,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Generate"), TextFont::from_font_size(22.0)));

            parent.spawn((
                PreviewImage,
                Node {
                    width: Val::Px(PREVIEW_WIDTH),
                    height: Val::Px(PREVIEW_HEIGHT),
                    margin: UiRect::vertical(Val::Px(8.0)),
                    ..default()
                },
                ImageNode::default(),
            ));

            parent
                .spawn((
                    Button,
                    WorldGenButton::Back,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Back"), TextFont::from_font_size(22.0)));
        });
}

fn seed_label(seed: u64) -> String {
    format!("Seed: {seed} (click to randomize)")
}

type OnlyPresetLabel = (With<PresetLabel>, Without<SizeLabel>, Without<SeedLabel>);
type OnlySizeLabel = (With<SizeLabel>, Without<PresetLabel>, Without<SeedLabel>);
type OnlySeedLabel = (With<SeedLabel>, Without<PresetLabel>, Without<SizeLabel>);

#[allow(clippy::too_many_arguments)]
fn button_actions(
    buttons: Query<(&Interaction, &WorldGenButton), Changed<Interaction>>,
    mut settings: ResMut<WorldGenSettings>,
    mut preset_labels: Query<&mut Text, OnlyPresetLabel>,
    mut size_labels: Query<&mut Text, OnlySizeLabel>,
    mut seed_labels: Query<&mut Text, OnlySeedLabel>,
    mut images: ResMut<Assets<Image>>,
    mut preview: Query<&mut ImageNode, With<PreviewImage>>,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            WorldGenButton::CyclePreset => {
                settings.preset_index = (settings.preset_index + 1) % preset::ALL.len();
                if let Ok(mut text) = preset_labels.single_mut() {
                    text.0 = format!("Preset: {}", preset::ALL[settings.preset_index].name);
                }
            }
            WorldGenButton::CycleSize => {
                settings.size_index = (settings.size_index + 1) % MAP_SIZES.len();
                if let Ok(mut text) = size_labels.single_mut() {
                    text.0 = format!("Size: {}", MAP_SIZES[settings.size_index].0);
                }
            }
            WorldGenButton::RandomizeSeed => {
                // A fixed-increment LCG step is enough here: we just need a
                // different-looking seed on each click, not real entropy.
                settings.seed = settings
                    .seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                if let Ok(mut text) = seed_labels.single_mut() {
                    text.0 = seed_label(settings.seed);
                }
            }
            WorldGenButton::Generate => {
                let chosen_preset = preset::ALL[settings.preset_index];
                let (_, width, height) = MAP_SIZES[settings.size_index];
                let world = generate(width, height, settings.seed, &chosen_preset);
                let rgb_image = image_export::biome_map(
                    width,
                    height,
                    &world.elevation,
                    &world.hydrology,
                    &world.biome,
                );
                let handle = images.add(rgb_to_bevy_image(&rgb_image));
                if let Ok(mut node) = preview.single_mut() {
                    node.image = handle;
                }
            }
            WorldGenButton::Back => next_screen.set(MainMenuScreen::SoloMode),
        }
    }
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
