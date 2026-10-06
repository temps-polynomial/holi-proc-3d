//! HOLI: a colour-festival meadow in Bevy 0.19.
//!
//! Meadow + wind-swept grass + flowers, grown trees, rocks, bushes, fence, bunting,
//! balloons, powder cannons, sky, the camera look stack (HDR, ACES, grading, bloom,
//! vignette, bokeh depth of field) and a toggle panel. The powder clouds are a static
//! volume built on the CPU (see `powder.rs`).
//!
//! Controls: drag orbit, shift+drag pan, scroll zoom.
//! F1-F12 / K (or click the panel rows) toggle layers and effects; Tab hides the panel.

mod geometry;
mod materials;
mod powder;

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::camera::Hdr;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::light::NotShadowCaster;
use bevy::pbr::MaterialPlugin;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::post_process::effect_stack::Vignette;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::{ColorGrading, ColorGradingGlobal};

use geometry::{Rng, blob_mesh, forest, grass_mesh, leaf_texture};
use materials::*;

const CELL: f32 = 0.06;
const BOX: Vec3 = Vec3::new(powder::NX as f32 * CELL, powder::NY as f32 * CELL, powder::NZ as f32 * CELL);
const BOX_MIN: Vec3 = Vec3::new(-BOX.x / 2.0, 0.0, -BOX.z / 2.0);
const N_CANNONS: usize = 8;
const PALETTE: [[f32; 3]; 8] = [
    [1.00, 0.16, 0.55], // hot pink
    [1.00, 0.50, 0.05], // orange
    [1.00, 0.88, 0.08], // yellow
    [0.45, 0.92, 0.12], // lime
    [0.05, 0.85, 0.95], // cyan
    [0.15, 0.40, 1.00], // blue
    [0.55, 0.20, 1.00], // violet
    [0.95, 0.15, 0.95], // magenta
];
const SATURATION: f32 = 1.18;
const TEMPERATURE: f32 = 0.015;
const VIGNETTE: f32 = 0.35;
const BLOOM: f32 = 0.12;
const F_STOPS: f32 = 1.6;

fn sun_dir() -> Vec3 {
    Vec3::new(-0.85, 0.7, 0.05).normalize()
}

fn to_world(x: f32, y: f32, z: f32) -> Vec3 {
    BOX_MIN + Vec3::new(x, y, z) * CELL
}

/// Height of the ground at (x, z): the meadow, or the top of a hill ellipsoid.
fn ground_height(hills: &[(Vec3, Vec3)], x: f32, z: f32) -> f32 {
    hills
        .iter()
        .map(|(c, r)| {
            let (dx, dz) = ((x - c.x) / r.x, (z - c.z) / r.z);
            let inside = 1.0 - dx * dx - dz * dz;
            if inside > 0.0 { c.y + r.y * inside.sqrt() } else { 0.0 }
        })
        .fold(0.0, f32::max)
}

/// Cell position of cannon i's muzzle and its aim (inward and up).
fn cannon_layout(i: usize) -> (Vec3, Vec3) {
    let a = i as f32 / N_CANNONS as f32 * TAU + 0.2;
    let r = 58.0;
    let (cx, cz) = (powder::NX as f32 / 2.0, powder::NZ as f32 / 2.0);
    (
        Vec3::new(cx + a.cos() * r, 5.0, cz + a.sin() * r),
        Vec3::new(-a.cos() * 0.22, 1.0, -a.sin() * 0.22),
    )
}

const GRASS: usize = 0;
const TREES: usize = 1;
const POWDER: usize = 2;
const DECOR: usize = 3;
const ROCKS: usize = 4;
const STAGE: usize = 5;
const HILLS: usize = 6;
const SKY: usize = 7;
const DOF_T: usize = 8;
const BLOOM_T: usize = 9;
const GRADE_T: usize = 10;
const SHADOW_T: usize = 11;
const WIND_T: usize = 12;
const TOGGLES: [&str; 13] = [
    "Grass & flowers", "Trees", "Powder", "Fence, bunting & balloons", "Rocks & bushes",
    "Stage & cannons", "Hills", "Sky", "Depth of field", "Bloom", "Vignette & colour grade",
    "Shadows", "Grass wind",
];
const TOGGLE_KEYS: [KeyCode; 13] = [
    KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4, KeyCode::F5, KeyCode::F6, KeyCode::F7,
    KeyCode::F8, KeyCode::F9, KeyCode::F10, KeyCode::F11, KeyCode::F12, KeyCode::KeyK,
];
const KEY_LABELS: [&str; 13] = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "K"];

#[derive(Resource)]
struct Toggles {
    on: [bool; 13],
    dirty: bool,
    panel: bool,
}

impl Default for Toggles {
    fn default() -> Self {
        Self { on: [true; 13], dirty: true, panel: true }
    }
}

#[derive(Component, Default, Clone)]
struct Layer(usize);

#[derive(Component, Default, Clone)]
struct ToggleButton(usize);

#[derive(Component, Default, Clone)]
struct TogglePanel;

#[derive(Component, Default, Clone)]
struct Sun;

#[derive(Component, Default, Clone)]
struct Balloon {
    phase: f32,
    base_y: f32,
}

#[derive(Resource)]
struct GrassHandle(Handle<GrassMaterial>);

/// Minimal orbit camera: drag to rotate, shift+drag to pan, scroll to zoom.
#[derive(Component, Default, Clone)]
struct Orbit {
    distance: f32,
    yaw: f32,
    pitch: f32,
    target: Vec3,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            MaterialPlugin::<HoliVolume>::default(),
            MaterialPlugin::<HoliSky>::default(),
            MaterialPlugin::<NatureMaterial>::default(),
            MaterialPlugin::<GrassMaterial>::default(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.6, 0.75, 0.95)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.62, 0.75, 1.0),
            brightness: 900.0,
            ..default()
        })
        .init_resource::<Toggles>()
        .add_systems(Startup, (setup_volume, setup_world, setup_camera_and_ui))
        .add_systems(
            Update,
            (orbit_camera, focus, sway_grass, animate_balloons, toggle_input, apply_toggles.after(toggle_input)),
        )
        .run();
}

fn sampler(mode: ImageAddressMode) -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        address_mode_w: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    })
}

fn volume_image(w: usize, h: usize, d: usize, data: Vec<u8>, format: TextureFormat, mode: ImageAddressMode) -> Image {
    let mut img = Image::new(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: d as u32 },
        TextureDimension::D3,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = sampler(mode);
    img
}

/// A mesh with its material, shown or hidden by toggle `layer`.
fn part<M: Material>(mesh: Handle<Mesh>, material: Handle<M>, layer: usize) -> impl Scene {
    bsn! { Mesh3d(mesh) MeshMaterial3d::<M>(material) Layer(layer) }
}

fn setup_volume(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut vols: ResMut<Assets<HoliVolume>>,
    mut skies: ResMut<Assets<HoliSky>>,
) {
    let (muzzles, aims): (Vec<_>, Vec<_>) = (0..N_CANNONS).map(cannon_layout).unzip();
    let vol = powder::build(&muzzles, &aims, &PALETTE, 7);
    let (nx, ny, nz) = (powder::NX, powder::NY, powder::NZ);
    let color = images.add(volume_image(nx, ny, nz, vol.color, TextureFormat::Rgba8Unorm, ImageAddressMode::ClampToEdge));
    let light = images.add(volume_image(nx, ny, nz, vol.light, TextureFormat::Rg8Unorm, ImageAddressMode::ClampToEdge));
    let mut g = Rng::new(11);
    let noise: Vec<u8> = (0..64 * 64 * 64 * 4).map(|_| (g.next_u64() >> 56) as u8).collect();
    let detail = images.add(volume_image(64, 64, 64, noise, TextureFormat::Rgba8Unorm, ImageAddressMode::Repeat));

    let material = vols.add(HoliVolume {
        params: VolumeParams {
            box_min: BOX_MIN,
            cell: CELL,
            box_size: BOX,
            step_scale: 1.0,
            sun_dir: sun_dir(),
            sun_gain: 1.25,
            sky_gain: 0.55,
            density: 9.0,
            detail_amount: 0.85,
            detail_freq: 0.08,
        },
        vol: color,
        lvol: light,
        detail,
    });
    let sky = skies.add(HoliSky { params: SkyParams { sun_dir: sun_dir(), gain: 1.0 } });
    commands.spawn_scene_list(bsn_list![
        #PowderVolume part(meshes.add(Cuboid::new(BOX.x, BOX.y, BOX.z)), material, POWDER)
            Transform::from_xyz(0.0, BOX.y / 2.0, 0.0) NotShadowCaster,
        #Sky part(meshes.add(Sphere::new(700.0)), sky, SKY) NotShadowCaster,
    ]);
}

#[allow(clippy::too_many_arguments)]
fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std: ResMut<Assets<StandardMaterial>>,
    mut nature_mats: ResMut<Assets<NatureMaterial>>,
    mut grass_mats: ResMut<Assets<GrassMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut rng = Rng::new(3);
    let meadow = nature_mats.add(nature(0.0, [0.10, 0.30, 0.03], [0.42, 0.66, 0.10], 3.0, 0.4, 1.0));
    let hill = nature_mats.add(nature(0.0, [0.14, 0.36, 0.05], [0.45, 0.66, 0.14], 3.0, 0.4, 0.0));
    let bush = nature_mats.add(nature(3.0, [0.08, 0.26, 0.04], [0.40, 0.64, 0.12], 5.0, 0.6, 1.0));
    let rock = nature_mats.add(nature(2.0, [0.20, 0.20, 0.20], [0.52, 0.50, 0.47], 2.5, 0.5, 1.0));
    let bark = nature_mats.add(nature(4.0, [0.05, 0.035, 0.025], [0.17, 0.13, 0.095], 5.0, 0.35, 1.0));

    // meadow + rolling hills
    commands.spawn_scene(bsn! {
        #Meadow Mesh3d(asset_value(Cuboid::new(900.0, 0.2, 900.0))) MeshMaterial3d::<NatureMaterial>(meadow)
        Transform::from_xyz(0.0, -0.1, 0.0)
    });
    let hill_mesh = meshes.add(Sphere::new(1.0).mesh().ico(4).unwrap());
    let mut hills = Vec::new();
    for i in 0..14 {
        let a = i as f32 / 14.0 * TAU + rng.range(-0.2, 0.2);
        let r = rng.range(30.0, 60.0);
        let d = rng.range(70.0, 120.0);
        let (centre, radii) = (Vec3::new(a.cos() * d, -r * 0.78, a.sin() * d), Vec3::new(r * 1.6, r, r * 1.3));
        hills.push((centre, radii));
        commands.spawn_scene(bsn! {
            part(hill_mesh.clone(), hill.clone(), HILLS) Transform { translation: centre, scale: radii } NotShadowCaster
        });
    }

    // grass carpet with flowers, swayed by the wind vertex shader
    let grass = grass_mats.add(GrassMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.8,
            reflectance: 0.3,
            cull_mode: None,
            ..default()
        },
        extension: Grass { params: GrassParams { now: 0.0, wind: Vec4::new(0.8, 0.0, 0.6, 0.045) } },
    });
    commands.insert_resource(GrassHandle(grass.clone()));
    commands.spawn_scene(bsn! { #Grass part(meshes.add(grass_mesh(4)), grass, GRASS) NotShadowCaster });

    // stage + cannons
    commands.spawn_scene(bsn! {
        part(meshes.add(Cylinder::new(2.2, 0.16)), std.add(StandardMaterial {
            base_color: Color::srgb(0.93, 0.86, 0.74),
            perceptual_roughness: 0.75,
            ..default()
        }), STAGE)
        Transform::from_xyz(0.0, 0.08, 0.0)
    });
    let barrel = meshes.add(Cylinder::new(0.17, 0.75));
    let drum = meshes.add(Cylinder::new(0.32, 0.36));
    let drum_mat = std.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.94, 0.9),
        perceptual_roughness: 0.5,
        ..default()
    });
    for (i, col) in PALETTE.iter().enumerate() {
        let (pos, aim) = cannon_layout(i);
        let w = to_world(pos.x, 0.0, pos.z);
        let paint = std.add(StandardMaterial {
            base_color: Color::srgb(col[0], col[1], col[2]),
            perceptual_roughness: 0.3,
            ..default()
        });
        let d = aim.normalize();
        commands.spawn_scene_list(bsn_list![
            part(drum.clone(), drum_mat.clone(), STAGE) Transform::from_xyz(w.x, 0.18, w.z),
            part(barrel.clone(), paint, STAGE)
                Transform { translation: {Vec3::new(w.x, 0.36, w.z) + d * 0.3}, rotation: Quat::from_rotation_arc(Vec3::Y, d) },
        ]);
    }

    // trees: grown procedurally (branching bark + alpha-masked leaf cards)
    let mut spots: Vec<(f32, f32, f32)> =
        (0..9).map(|_| (rng.range(0.0, TAU), rng.range(10.5, 13.0), rng.range(1.0, 1.35))).collect();
    spots.extend((0..22).map(|i| (i as f32 / 22.0 * TAU + rng.range(-0.12, 0.12), rng.range(15.5, 24.0), rng.range(1.1, 1.7))));
    spots.extend((0..45).map(|_| (rng.range(0.0, TAU), rng.range(32.0, 70.0), rng.range(1.4, 2.4))));
    let real: Vec<(Vec3, f32, usize)> = spots
        .iter()
        .enumerate()
        .filter(|(_, (a, r, _))| !(*r < 13.5 && (a - 0.4).sin().atan2((a - 0.4).cos()).abs() < 0.9))
        .map(|(i, &(a, r, s))| {
            let (x, z) = (a.cos() * r, a.sin() * r);
            (Vec3::new(x, ground_height(&hills, x, z), z), s * 1.2, i)
        })
        .collect();
    let f = forest(&real, 5);
    let leaf_tex = Image::new(
        Extent3d { width: 128, height: 128, depth_or_array_layers: 1 },
        TextureDimension::D2,
        leaf_texture(128, 2),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.spawn_scene_list(bsn_list![
        #Branches part(meshes.add(f.bark_mesh()), bark, TREES),
        #Leaves part(meshes.add(f.leaf_mesh()), std.add(StandardMaterial {
            base_color_texture: Some(images.add(leaf_tex)),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            perceptual_roughness: 0.9,
            reflectance: 0.15,
            diffuse_transmission: 0.15,
            ..default()
        }), TREES),
    ]);

    // rocks and flowering bushes (each set merged into one mesh)
    let rocks: Vec<_> = (0..14)
        .map(|_| {
            let (a, r, s) = (rng.range(0.0, TAU), rng.range(4.5, 11.5), rng.range(0.25, 0.7));
            (Vec3::new(a.cos() * r, 0.1 * s, a.sin() * r), Vec3::new(s * 1.3, s * 0.6, s), rng.range(0.0, TAU))
        })
        .collect();
    commands.spawn_scene(bsn! { part(meshes.add(blob_mesh(&rocks)), rock, ROCKS) });
    let mut bushes = Vec::new();
    for _ in 0..14 {
        let (a, r) = (rng.range(0.0, TAU), rng.range(5.0, 12.0));
        let (cx, cz) = (a.cos() * r, a.sin() * r);
        for _ in 0..rng.int(3, 6) {
            let s = rng.range(0.3, 0.55);
            bushes.push((
                Vec3::new(cx + rng.range(-0.5, 0.5), s * 0.6, cz + rng.range(-0.5, 0.5)),
                Vec3::new(s, s * 0.85, s),
                0.0,
            ));
        }
    }
    commands.spawn_scene(bsn! { part(meshes.add(blob_mesh(&bushes)), bush, ROCKS) });

    // wooden fence ring
    let wood = std.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.28, 0.17),
        perceptual_roughness: 0.85,
        ..default()
    });
    let posts = 48;
    let fence_r = 14.0;
    let seg = 2.0 * fence_r * (PI / posts as f32).sin();
    let post = meshes.add(Cuboid::new(0.12, 1.0, 0.12));
    let rail = meshes.add(Cuboid::new(seg + 0.1, 0.07, 0.05));
    for i in 0..posts {
        let a = i as f32 / posts as f32 * TAU;
        commands.spawn_scene(bsn! { part(post.clone(), wood.clone(), DECOR) Transform::from_xyz(a.cos() * fence_r, 0.5, a.sin() * fence_r) });
        let am = (i as f32 + 0.5) / posts as f32 * TAU;
        let mid = Vec3::new(am.cos(), 0.0, am.sin()) * fence_r * (PI / posts as f32).cos();
        for y in [0.45, 0.8] {
            commands.spawn_scene(bsn! {
                part(rail.clone(), wood.clone(), DECOR)
                Transform { translation: Vec3::new(mid.x, y, mid.z), rotation: Quat::from_rotation_y(-am + PI / 2.0) }
            });
        }
    }

    // bunting: pennant strings sagging between tall poles (outside the camera orbit)
    let pole = meshes.add(Cylinder::new(0.05, 3.4));
    let string = meshes.add(Cylinder::new(0.008, 1.0));
    let flag = meshes.add(Triangle3d::new(Vec3::new(-0.13, 0.0, 0.0), Vec3::new(0.13, 0.0, 0.0), Vec3::new(0.0, -0.3, 0.0)));
    let white = std.add(StandardMaterial { base_color: Color::srgb(0.95, 0.95, 0.95), ..default() });
    let flag_mats: Vec<_> = PALETTE
        .iter()
        .map(|c| {
            std.add(StandardMaterial {
                base_color: Color::srgb(c[0], c[1], c[2]),
                perceptual_roughness: 0.6,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let (poles, pr) = (10, 12.8);
    let tops: Vec<Vec3> = (0..poles)
        .map(|i| {
            let a = i as f32 / poles as f32 * TAU + 0.1;
            let p = Vec3::new(a.cos() * pr, 0.0, a.sin() * pr);
            commands.spawn_scene(bsn! { part(pole.clone(), wood.clone(), DECOR) Transform::from_xyz(p.x, 1.7, p.z) });
            Vec3::new(p.x, 3.35, p.z)
        })
        .collect();
    let mut k = 0;
    for i in 0..poles {
        let (p0, p1) = (tops[i], tops[(i + 1) % poles]);
        let sag = |t: f32| p0 + (p1 - p0) * t - Vec3::new(0.0, 0.7 * 4.0 * t * (1.0 - t), 0.0);
        let yaw = (-(p1.z - p0.z)).atan2(p1.x - p0.x);
        let mut prev = p0;
        for j in 1..=10 {
            let cur = sag(j as f32 / 10.0);
            let d = cur - prev;
            commands.spawn_scene(bsn! {
                part(string.clone(), white.clone(), DECOR) Transform {
                    translation: {(cur + prev) * 0.5},
                    rotation: Quat::from_rotation_arc(Vec3::Y, d.normalize()),
                    scale: Vec3::new(1.0, d.length(), 1.0),
                }
            });
            prev = cur;
        }
        let nflags = ((p1 - p0).length() / 0.42) as usize;
        for j in 1..nflags {
            commands.spawn_scene(bsn! {
                part(flag.clone(), flag_mats[k % flag_mats.len()].clone(), DECOR) NotShadowCaster
                Transform { translation: sag(j as f32 / nflags as f32), rotation: Quat::from_rotation_y(yaw) }
            });
            k += 1;
        }
    }

    // balloons on strings
    let balloon = meshes.add(Sphere::new(0.26));
    for i in 0..14 {
        let a = i as f32 / 14.0 * TAU + rng.range(-0.15, 0.15);
        let r = rng.range(4.2, 9.0);
        let y = rng.range(1.4, 2.8);
        let c = PALETTE[(i * 3) % PALETTE.len()];
        let mat = std.add(StandardMaterial {
            base_color: Color::srgb(c[0], c[1], c[2]),
            perceptual_roughness: 0.2,
            clearcoat: 0.9,
            ..default()
        });
        let length = y / 1.15;
        commands.spawn_scene(bsn! {
            part(balloon.clone(), mat, DECOR) Balloon { phase: {rng.range(0.0, TAU)}, base_y: y }
            Transform { translation: Vec3::new(a.cos() * r, y, a.sin() * r), scale: Vec3::new(1.0, 1.15, 1.0) }
            Children [
                Mesh3d({string.clone()}) MeshMaterial3d::<StandardMaterial>({white.clone()})
                Transform { translation: Vec3::new(0.0, -0.25 - length / 2.0, 0.0), scale: Vec3::new(1.0, length, 1.0) }
            ]
        });
    }

    commands.spawn_scene(bsn! {
        Sun DirectionalLight { illuminance: 16000.0, shadow_maps_enabled: true, color: Color::srgb(1.0, 0.88, 0.7) }
        template_value(Transform::from_translation(sun_dir() * 30.0).looking_at(Vec3::ZERO, Vec3::Y))
    });
}

fn setup_camera_and_ui(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Camera3d Hdr DepthPrepass template_value(Tonemapping::AcesFitted) Transform
        ColorGrading { global: ColorGradingGlobal { post_saturation: SATURATION, temperature: TEMPERATURE } }
        Bloom::NATURAL Bloom { intensity: BLOOM } Vignette { intensity: VIGNETTE, radius: 0.85, smoothness: 3.0 }
        DepthOfField { mode: DepthOfFieldMode::Bokeh, focal_distance: 11.5, sensor_height: 0.32, aperture_f_stops: F_STOPS }
        Orbit { distance: 11.5, yaw: 0.4, pitch: 0.17, target: Vec3::new(0.0, 1.5, 0.0) }
    });

    commands.spawn_scene(bsn! {
        Text("HOLI (Rust Bevy 0.19) - static powder\ndrag orbit, shift+drag pan, scroll zoom") TextFont { font_size: {16.0} }
        Node { position_type: PositionType::Absolute, top: px(12), left: px(14) } BackgroundColor(Color::srgba(0.1, 0.1, 0.25, 0.3))
    });
    commands.spawn_scene(bsn! {
        TogglePanel BackgroundColor(Color::srgba(0.05, 0.08, 0.18, 0.45))
        Node {
            position_type: PositionType::Absolute, bottom: px(12), right: px(12),
            flex_direction: FlexDirection::Column, row_gap: px(2), padding: UiRect::all(px(6)),
        }
        Children [
            Text("toggles (Tab hides)") TextFont { font_size: {13.0} } TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
            {(0..TOGGLES.len()).map(|i| bsn! {
                Node { padding: UiRect::axes(px(8), px(3)) } Button ToggleButton(i) BackgroundColor(Color::srgba(0.2, 0.6, 0.3, 0.55))
                Children [Text("") TextFont { font_size: {14.0} } ToggleButton(i)]
            }).collect::<Vec<_>>()}
        ]
    });
}

fn orbit_camera(
    mut cams: Query<(&mut Transform, &mut Orbit)>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
) {
    for (mut tr, mut o) in &mut cams {
        if buttons.pressed(MouseButton::Left) {
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                let right = tr.right();
                let up = tr.up();
                let k = o.distance * 0.001;
                o.target -= right * motion.delta.x * k;
                o.target += up * motion.delta.y * k;
            } else {
                o.yaw -= motion.delta.x * 0.003;
                o.pitch = (o.pitch + motion.delta.y * 0.003).clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
            }
        }
        o.distance = (o.distance * (1.0 - scroll.delta.y * 0.1)).clamp(8.0, 60.0);
        o.yaw += time.delta_secs() * 0.04; // slow drift
        let eye = o.target
            + Vec3::new(o.pitch.cos() * o.yaw.sin(), o.pitch.sin(), o.pitch.cos() * o.yaw.cos()) * o.distance;
        *tr = Transform::from_translation(eye).looking_at(o.target, Vec3::Y);
    }
}

fn focus(mut cams: Query<(&Orbit, &mut DepthOfField)>, toggles: Res<Toggles>) {
    for (o, mut dof) in &mut cams {
        dof.focal_distance = o.distance;
        dof.aperture_f_stops = if toggles.on[DOF_T] { F_STOPS } else { 1000.0 };
    }
}

fn sway_grass(time: Res<Time>, toggles: Res<Toggles>, handle: Res<GrassHandle>, mut mats: ResMut<Assets<GrassMaterial>>) {
    if let Some(mut m) = mats.get_mut(&handle.0) {
        m.extension.params.now = time.elapsed_secs();
        m.extension.params.wind.w = if toggles.on[WIND_T] { 0.045 } else { 0.0 };
    }
}

fn animate_balloons(time: Res<Time>, mut balloons: Query<(&mut Transform, &Balloon)>) {
    let t = time.elapsed_secs();
    for (mut tr, b) in &mut balloons {
        tr.translation.y = b.base_y + 0.12 * (t * 0.9 + b.phase).sin();
        tr.rotation = Quat::from_rotation_z(0.08 * (t * 0.7 + b.phase * 1.3).sin());
    }
}

fn toggle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut toggles: ResMut<Toggles>,
    buttons: Query<(&Interaction, &ToggleButton), Changed<Interaction>>,
    mut panels: Query<&mut Node, With<TogglePanel>>,
) {
    let mut flips: Vec<usize> = TOGGLE_KEYS.iter().enumerate().filter(|(_, k)| keys.just_pressed(**k)).map(|(i, _)| i).collect();
    if keys.just_pressed(KeyCode::KeyF) {
        flips.push(DOF_T);
    }
    for (interaction, tb) in &buttons {
        if *interaction == Interaction::Pressed {
            flips.push(tb.0);
        }
    }
    for i in flips {
        toggles.on[i] = !toggles.on[i];
        toggles.dirty = true;
    }
    if keys.just_pressed(KeyCode::Tab) {
        toggles.panel = !toggles.panel;
        for mut node in &mut panels {
            node.display = if toggles.panel { Display::Flex } else { Display::None };
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_toggles(
    mut toggles: ResMut<Toggles>,
    mut layers: Query<(&Layer, &mut Visibility)>,
    mut cams: Query<(&mut Bloom, &mut Vignette, &mut ColorGrading)>,
    mut suns: Query<&mut DirectionalLight, With<Sun>>,
    mut labels: Query<(&mut Text, &ToggleButton)>,
    mut rows: Query<(&mut BackgroundColor, &ToggleButton), Without<Text>>,
) {
    if !toggles.dirty {
        return;
    }
    toggles.dirty = false;
    let on = toggles.on;
    for (layer, mut vis) in &mut layers {
        *vis = if on[layer.0] { Visibility::Inherited } else { Visibility::Hidden };
    }
    for (mut bloom, mut vignette, mut grade) in &mut cams {
        bloom.intensity = if on[BLOOM_T] { BLOOM } else { 0.0 };
        vignette.intensity = if on[GRADE_T] { VIGNETTE } else { 0.0 };
        grade.global.post_saturation = if on[GRADE_T] { SATURATION } else { 1.0 };
        grade.global.temperature = if on[GRADE_T] { TEMPERATURE } else { 0.0 };
    }
    for mut light in &mut suns {
        light.shadow_maps_enabled = on[SHADOW_T];
    }
    for (mut text, tb) in &mut labels {
        text.0 = format!("[{}]  {}   {}", KEY_LABELS[tb.0], if on[tb.0] { "on " } else { "off" }, TOGGLES[tb.0]);
    }
    for (mut bg, tb) in &mut rows {
        bg.0 = if on[tb.0] { Color::srgba(0.2, 0.6, 0.3, 0.55) } else { Color::srgba(0.35, 0.35, 0.4, 0.45) };
    }
}
