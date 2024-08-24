use bevy::audio::Volume;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::reflect::Enum;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use bevy::window::PresentMode;

// Define constants for key codes and corresponding colors.
// The colors are defined using the srgb color space.
const KEY_CODES: [KeyCode; 3] = [KeyCode::KeyQ, KeyCode::KeyW, KeyCode::KeyC];
const KEY_COLORS: [Color; 3] = [
    Color::srgb(1.0, 0.0, 0.0), // Red color for KeyQ
    Color::srgb(0.0, 1.0, 0.0), // Green color for KeyW
    Color::srgb(0.0, 0.0, 1.0), // Blue color for KeyC
];

// Constants for key size, spacing between keys, and trail speed.
// These values define the visual properties and behavior of the keys and trails.
const KEY_SIZE: f32 = 80.0;
const KEY_SPACING: f32 = 10.0;
const TRAIL_SPEED: f32 = 250.0;
const TRAIL_SCALE_SPEED: f32 = TRAIL_SPEED / 2.0; // Trail grows at half the speed of its movement

// Calculate the total width occupied by all keys and the starting X position
// for placing the first key. This helps in positioning the keys at the center of the window.
const TOTAL_WIDTH: f32 = (KEY_SIZE * KEY_CODES.len() as f32) + (KEY_SPACING * (KEY_CODES.len() - 1) as f32);
const START_X: f32 = -TOTAL_WIDTH / 2.0;

// Define the window dimensions, ensuring it comfortably fits the keys and allows space for trails.
const WINDOW_WIDTH: f32 = TOTAL_WIDTH + KEY_SPACING * 2.0;
const WINDOW_HEIGHT: f32 = 600.0;

fn main() {
    // Set up the application with a custom window plugin and add the necessary systems.
    App::new().add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Key Cube".to_string(), // Window title
            resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(), // Window size
            resizable: false, // Disable window resizing
            present_mode: PresentMode::AutoVsync, // VSync to avoid screen tearing
            enabled_buttons: bevy::window::EnabledButtons {
                maximize: false,
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    })).add_systems(Startup, setup_graphics) // Set up the initial graphics and scene
        .add_systems(Update, (update_keys, update_trails)) // Update the keys and trails each frame
        .run();
}

// Component to uniquely identify each key by its KeyCode.
#[derive(Component)]
struct KeyID(KeyCode);

// Component to represent a trail left by a key press, tracking its state and associated key.
#[derive(Component)]
struct Trail {
    key: KeyCode,
    is_active: bool,
}

// Component to keep track of the number of times a key is pressed.
#[derive(Component)]
struct ClicksCount(usize);

// System to set up the initial scene, including the camera, keys, and UI elements.
fn setup_graphics(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut windows: Query<&mut Window>,
) {
    let window = windows.single_mut();
    let key_y = -window.resolution.height() / 2.0 + KEY_SIZE / 2.0; // Position the keys near the bottom of the window

    // Spawn a 2D camera with tonemapping to handle rendering.
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                ..Default::default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            transform: Transform::from_xyz(0.0, 0.0, 1.0), // Position the camera in the center
            ..Default::default()
        },
    ));

    // Prepare the positions and properties for each key based on the predefined constants.
    let mut keys = Vec::new();
    for (i, key_code) in KEY_CODES.iter().enumerate() {
        let key_spacing = if i == 0 { 0.0 } else { KEY_SPACING };
        let key_x = START_X + i as f32 * (KEY_SIZE + key_spacing) + KEY_SIZE / 2.0;
        keys.push((*key_code, KEY_COLORS[i], Vec2::new(key_x, key_y)));
    }

    // Spawn the keys with their respective materials and positions.
    for (key_code, color, position) in &keys {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(Rectangle::new(KEY_SIZE, KEY_SIZE))), // Create a square mesh for the key
                material: materials.add(ColorMaterial::from(*color)), // Set the color of the key
                transform: Transform::from_translation(position.extend(0.0)), // Position the key in 2D space
                ..Default::default()
            },
            KeyID(*key_code), // Assign the KeyID component to identify the key
        )).with_children(|parent| {
            // Add a text child to display the key's code (e.g., Q, W, C) on the key.
            parent.spawn(Text2dBundle {
                text: Text::from_section(
                    key_code.variant_name().replace("Key", ""), // Remove "Key" prefix for display
                    TextStyle {
                        font: Default::default(),
                        font_size: 32.0, // Large font for key code
                        color: Color::WHITE,
                    },
                ),
                transform: Transform::from_translation(Vec3::Z), // Center the text on the key
                ..Default::default()
            });
        }).with_children(|parent| {
            // Add a second text child to display the number of times the key is pressed.
            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "0", // Initial click count
                        TextStyle {
                            font: Default::default(),
                            font_size: 16.0, // Smaller font for click count
                            color: Color::WHITE,
                        },
                    ).with_justify(JustifyText::Center),
                    transform: Transform::from_translation(Vec3::new(0.0, -26.0, Vec3::Z.z)), // Position below the key code text
                    ..Default::default()
                },
                ClicksCount(0), // Initialize click count to 0
            ));
        });
    }
}

// System to handle key presses, update visual states, and spawn trails.
fn update_keys(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut key_query: Query<(&KeyID, &Handle<ColorMaterial>, &Transform, &mut Children)>,
    mut text_query: Query<(&mut Text, &mut ClicksCount)>,
) {
    let mut key_presses = Vec::new(); // Store registered key presses

    // Iterate over all keys to check if they are pressed and update their visual state.
    for (key_id, material_handle, transform, children) in key_query.iter_mut() {
        let is_pressed = input.pressed(key_id.0);
        let just_pressed = input.just_pressed(key_id.0);

        // Modify the key's appearance based on its press state.
        if let Some(material) = materials.get_mut(material_handle) {
            if is_pressed {
                material.color.set_alpha(0.5); // Semi-transparent when pressed
            } else {
                material.color.set_alpha(1.0); // Full opacity when released
            }
        }

        // Register the key press if it was just pressed and update the click count display.
        if just_pressed {
            key_presses.push((key_id.0, *transform)); // Store the key press and its position

            for &child in children.iter() {
                if let Ok((mut text, mut clicks_count)) = text_query.get_mut(child) {
                    clicks_count.0 += 1; // Increment click count
                    text.sections[0].value = clicks_count.0.to_string(); // Update the displayed click count
                }
            }
        }
    }

    // Spawn a trail for each registered key press.
    for (key_code, transform) in key_presses {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(Rectangle::new(80.0, 1.0))), // Thin rectangle for the trail
                material: materials.add(ColorMaterial {
                    color: Color::srgb(1.0, 1.0, 1.0), // White trail color
                    ..Default::default()
                }),
                transform: transform * Transform::from_translation(Vec3::new(0.0, KEY_SIZE / 2.0, 0.0)), // Start trail from top of key
                ..Default::default()
            },
            Trail {
                key: key_code,
                is_active: true, // Trail is active upon creation
            },
            AudioBundle {
                source: asset_server.load("audio/hitsound.wav"),
                settings: PlaybackSettings::ONCE.with_volume(Volume::new(0.2)),
            },
        ));
    }
}

// System to update the trails, moving them upwards and despawning them when they exit the window.
fn update_trails(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
    mut trail_query: Query<(Entity, &mut Transform, &mut Trail)>,
) {
    for (entity, mut trail_transform, mut trail) in trail_query.iter_mut() {
        // Deactivate the trail if the corresponding key is released.
        if input.just_released(trail.key) {
            trail.is_active = false;
        }

        // Update the trail's position and size based on its active state.
        if trail.is_active {
            trail_transform.translation.y += time.delta_seconds() * TRAIL_SCALE_SPEED;
            if input.pressed(trail.key) {
                trail_transform.scale.y += time.delta_seconds() * TRAIL_SPEED;
            }
        } else {
            trail_transform.translation.y += time.delta_seconds() * TRAIL_SPEED;
        }

        // De-spawn the trail if it has moved out of the window's bounds.
        let window = windows.single_mut();
        if !input.pressed(trail.key) && trail_transform.translation.y - trail_transform.scale.y / 2.0 > window.resolution.height() / 2.0
        {
            commands.entity(entity).despawn();
        }
    }
}