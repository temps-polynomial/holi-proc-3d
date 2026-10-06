//! Caméra orbitale pilotée à la souris, et sa chaîne de post-traitement : HDR, courbe ACES,
//! étalonnage, halo, vignette, profondeur de champ (ces effets suivent les bascules).

use std::f32::consts::PI;

use bevy::camera::Hdr;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::post_process::effect_stack::Vignette;
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal};

use crate::bascules::{Bascule, Bascules, SaisieBascules};

/// Plugin : regroupe ressources et systèmes d'un domaine ; ajouté à l'App dans main.rs.
// Book : https://bevy.org/learn/book/modular-architecture/plugins/
// Doc : https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html
pub struct PluginCamera;

impl Plugin for PluginCamera {
    fn build(&self, app: &mut App) {
        // Startup : une fois au lancement ; Update : à chaque image.
        // after + run_if : après la saisie, et seulement si Bascules a changé.
        // Book : https://bevy.org/learn/book/the-game-loop/schedules/#the-standard-bevy-schedules
        // Book : https://bevy.org/learn/book/control-flow/run-conditions/#run-conditions
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_systems
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.run_if
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/common_conditions/fn.resource_changed.html
        app.add_systems(Startup, creer_camera).add_systems(
            Update,
            (
                camera_orbitale,
                mise_au_point,
                appliquer_effets.after(SaisieBascules).run_if(resource_changed::<Bascules>),
            ),
        );
    }
}

const SATURATION: f32 = 1.18;
const TEMPERATURE: f32 = 0.015;
const VIGNETTE: f32 = 0.35;
const HALO: f32 = 0.12;
/// Nombre d'ouverture (f/1.6) de la profondeur de champ.
const OUVERTURE: f32 = 1.6;

/// Caméra orbitale minimale : glisser pour tourner, maj+glisser pour déplacer, molette pour zoomer.
// Component : donnée attachée à une entité. Default + Clone : requis par `bsn!`.
// Book : https://bevy.org/learn/book/storing-data/entities-components/#defining-components
// Doc : https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html
#[derive(Component, Default, Clone)]
struct Orbite {
    distance: f32,
    lacet: f32,
    tangage: f32,
    cible: Vec3,
}

/// Système Startup : la caméra et ses effets.
// Commands + spawn_scene(bsn! {...}) : crée une entité portant les composants listés.
// Camera3d : caméra perspective. Hdr : rendu en flottant (exigé par Bloom).
// DepthPrepass : profondeur écrite avant la passe principale (lue par holi_poudre.wgsl).
// template_value(x) : remplace entièrement le composant par x (ici Tonemapping::AcesFitted,
// courbe HDR -> écran de style cinéma). Transform : position et orientation (mises à jour par
// camera_orbitale). ColorGrading : saturation et température globales. Bloom : halo autour des
// zones claires. Vignette : assombrit les bords. DepthOfField::Bokeh : flou hors du plan de netteté.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L33-L36
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/tonemapping.rs#L290-L303
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_prepass.rs#L50-L55
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/color_grading.rs#L322-L327
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/bloom_3d.rs#L27-L36
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/post_processing.rs#L100-L101
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/depth_of_field.rs#L219-L229
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
// Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
// Doc : https://docs.rs/bevy/latest/bevy/scene/fn.template_value.html
// Doc : https://docs.rs/bevy/latest/bevy/camera/struct.Camera3d.html
// Doc : https://docs.rs/bevy/latest/bevy/camera/struct.Hdr.html
// Doc : https://docs.rs/bevy/latest/bevy/core_pipeline/prepass/struct.DepthPrepass.html
// Doc : https://docs.rs/bevy/latest/bevy/core_pipeline/tonemapping/enum.Tonemapping.html
// Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html
// Doc : https://docs.rs/bevy/latest/bevy/render/view/struct.ColorGrading.html
// Doc : https://docs.rs/bevy/latest/bevy/render/view/struct.ColorGradingGlobal.html
// Doc : https://docs.rs/bevy/latest/bevy/post_process/bloom/struct.Bloom.html#associatedconstant.NATURAL
// Doc : https://docs.rs/bevy/latest/bevy/post_process/effect_stack/struct.Vignette.html
// Doc : https://docs.rs/bevy/latest/bevy/post_process/dof/struct.DepthOfField.html
// Doc : https://docs.rs/bevy/latest/bevy/post_process/dof/enum.DepthOfFieldMode.html
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
fn creer_camera(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Camera3d Hdr DepthPrepass template_value(Tonemapping::AcesFitted) Transform
        ColorGrading { global: ColorGradingGlobal { post_saturation: SATURATION, temperature: TEMPERATURE } }
        // Bloom::NATURAL puis Bloom { intensity } : préréglage, puis seul `intensity` est modifié.
        Bloom::NATURAL Bloom { intensity: HALO } Vignette { intensity: VIGNETTE, radius: 0.85, smoothness: 3.0 }
        DepthOfField { mode: DepthOfFieldMode::Bokeh, focal_distance: 11.5, sensor_height: 0.32, aperture_f_stops: OUVERTURE }
        Orbite { distance: 11.5, lacet: 0.4, tangage: 0.17, cible: Vec3::new(0.0, 1.5, 0.0) }
    });
}

/// Système Update : rotation, déplacement et zoom de la caméra à la souris, plus une lente dérive.
// Single : exactement une entité correspond (sinon le système est sauté).
// Res<T> : accès en lecture à une ressource.
// ButtonInput<T> : état des boutons/touches (pressed = maintenu).
// AccumulatedMouseMotion/Scroll : déplacement de la souris et de la molette cumulés sur l'image
// (pas de multiplication par delta_secs, voir l'exemple camera_orbit).
// Time : delta_secs = durée de l'image (rend la dérive indépendante du nombre d'images/s).
// Book : https://bevy.org/learn/book/storing-data/queries/#working-with-singleton-entities
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Book : https://bevy.org/learn/book/the-game-loop/game-time/#frame-rate-independence-and-delta-time
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/input/mouse_input.rs#L15-L43
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/camera/camera_orbit.rs#L116-L123
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Single.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.Res.html
// Doc : https://docs.rs/bevy/latest/bevy/input/struct.ButtonInput.html
// Doc : https://docs.rs/bevy/latest/bevy/input/mouse/enum.MouseButton.html
// Doc : https://docs.rs/bevy/latest/bevy/input/keyboard/enum.KeyCode.html
// Doc : https://docs.rs/bevy/latest/bevy/input/mouse/struct.AccumulatedMouseMotion.html
// Doc : https://docs.rs/bevy/latest/bevy/input/mouse/struct.AccumulatedMouseScroll.html
// Doc : https://docs.rs/bevy/latest/bevy/time/struct.Time.html#method.delta_secs
fn camera_orbitale(
    camera: Single<(&mut Transform, &mut Orbite)>,
    boutons: Res<ButtonInput<MouseButton>>,
    touches: Res<ButtonInput<KeyCode>>,
    mouvement: Res<AccumulatedMouseMotion>,
    molette: Res<AccumulatedMouseScroll>,
    temps: Res<Time>,
) {
    let (mut tr, mut o) = camera.into_inner();
    if boutons.pressed(MouseButton::Left) {
        if touches.pressed(KeyCode::ShiftLeft) || touches.pressed(KeyCode::ShiftRight) {
            // right()/up() : axes locaux de la caméra dans le monde.
            // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.right
            let droite = tr.right();
            let haut = tr.up();
            let k = o.distance * 0.001;
            o.cible -= droite * mouvement.delta.x * k;
            o.cible += haut * mouvement.delta.y * k;
        } else {
            o.lacet -= mouvement.delta.x * 0.003;
            o.tangage = (o.tangage + mouvement.delta.y * 0.003).clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
        }
    }
    o.distance = (o.distance * (1.0 - molette.delta.y * 0.1)).clamp(8.0, 60.0);
    o.lacet += temps.delta_secs() * 0.04; // lente dérive
    let oeil = o.cible
        + Vec3::new(o.tangage.cos() * o.lacet.sin(), o.tangage.sin(), o.tangage.cos() * o.lacet.cos()) * o.distance;
    // from_translation : position seule ; looking_at(cible, haut) : oriente -Z local vers la cible.
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.from_translation
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.looking_at
    *tr = Transform::from_translation(oeil).looking_at(o.cible, Vec3::Y);
}

/// Système Update : plan de netteté sur la cible ; flou désactivé par une très grande ouverture.
fn mise_au_point(camera: Single<(&Orbite, &mut DepthOfField)>, bascules: Res<Bascules>) {
    let (o, mut pdc) = camera.into_inner();
    pdc.focal_distance = o.distance;
    pdc.aperture_f_stops = if bascules.actif(Bascule::Profondeur) { OUVERTURE } else { 1000.0 };
}

/// Système Update (seulement quand Bascules change) : halo, vignette et étalonnage.
/// Une intensité nulle désactive l'effet (voir la doc de chaque champ).
fn appliquer_effets(bascules: Res<Bascules>, camera: Single<(&mut Bloom, &mut Vignette, &mut ColorGrading)>) {
    let (mut halo, mut vignette, mut etalonnage) = camera.into_inner();
    let etal = bascules.actif(Bascule::Etalonnage);
    halo.intensity = if bascules.actif(Bascule::Halo) { HALO } else { 0.0 };
    vignette.intensity = if etal { VIGNETTE } else { 0.0 };
    etalonnage.global.post_saturation = if etal { SATURATION } else { 1.0 };
    etalonnage.global.temperature = if etal { TEMPERATURE } else { 0.0 };
}
