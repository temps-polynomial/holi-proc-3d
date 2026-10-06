//! HOLI : une prairie de fête des couleurs avec Bevy 0.19.
//!
//! Prairie + herbe agitée par le vent + fleurs, arbres générés, rochers, buissons, clôture,
//! fanions, ballons, canons à poudre, ciel, la chaîne d'image de la caméra (HDR, ACES,
//! étalonnage, halo, vignette, flou de profondeur de champ) et un panneau de bascules.
//! Les nuages de poudre sont un volume statique calculé sur le CPU (voir `poudre.rs`).
//!
//! Commandes : glisser = orbite, maj+glisser = déplacer, molette = zoom.
//! F1-F12 / K (ou clic sur les lignes du panneau) basculent calques et effets ; Tab masque le panneau.

// Modules Rust du projet (un fichier chacun).
// Book : https://bevy.org/learn/book/modular-architecture/project-organization/#modules
mod geometrie;
mod materiaux;
mod poudre;

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
// Prélude : importe les types les plus courants (App, Commands, Query, Transform...).
// Doc : https://docs.rs/bevy/latest/bevy/prelude/index.html
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::{ColorGrading, ColorGradingGlobal};

use geometrie::{Alea, foret, maillage_ellipsoides, maillage_herbe, texture_feuille};
use materiaux::*;

/// Taille d'une cellule du volume de poudre (m).
const CELLULE: f32 = 0.06;
// Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize, length...) sur la même page.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
const BOITE: Vec3 = Vec3::new(poudre::NX as f32 * CELLULE, poudre::NY as f32 * CELLULE, poudre::NZ as f32 * CELLULE);
const BOITE_MIN: Vec3 = Vec3::new(-BOITE.x / 2.0, 0.0, -BOITE.z / 2.0);
const N_CANONS: usize = 8;
const PALETTE: [[f32; 3]; 8] = [
    [1.00, 0.16, 0.55], // rose vif
    [1.00, 0.50, 0.05], // orange
    [1.00, 0.88, 0.08], // jaune
    [0.45, 0.92, 0.12], // citron vert
    [0.05, 0.85, 0.95], // cyan
    [0.15, 0.40, 1.00], // bleu
    [0.55, 0.20, 1.00], // violet
    [0.95, 0.15, 0.95], // magenta
];
const SATURATION: f32 = 1.18;
const TEMPERATURE: f32 = 0.015;
const VIGNETTE: f32 = 0.35;
const HALO: f32 = 0.12;
/// Nombre d'ouverture (f/1.6) de la profondeur de champ.
const OUVERTURE: f32 = 1.6;

fn dir_soleil() -> Vec3 {
    Vec3::new(-0.85, 0.7, 0.05).normalize()
}

/// Coordonnées de cellule du volume -> coordonnées du monde.
fn vers_monde(x: f32, y: f32, z: f32) -> Vec3 {
    BOITE_MIN + Vec3::new(x, y, z) * CELLULE
}

/// Hauteur du sol en (x, z) : la prairie, ou le sommet d'un ellipsoïde de colline.
fn hauteur_sol(collines: &[(Vec3, Vec3)], x: f32, z: f32) -> f32 {
    collines
        .iter()
        .map(|(c, r)| {
            let (dx, dz) = ((x - c.x) / r.x, (z - c.z) / r.z);
            let dedans = 1.0 - dx * dx - dz * dz;
            if dedans > 0.0 { c.y + r.y * dedans.sqrt() } else { 0.0 }
        })
        .fold(0.0, f32::max)
}

/// Position (en cellules) de la bouche du canon i et sa visée (vers l'intérieur et le haut).
fn disposition_canon(i: usize) -> (Vec3, Vec3) {
    let a = i as f32 / N_CANONS as f32 * TAU + 0.2;
    let r = 58.0;
    let (cx, cz) = (poudre::NX as f32 / 2.0, poudre::NZ as f32 / 2.0);
    (
        Vec3::new(cx + a.cos() * r, 5.0, cz + a.sin() * r),
        Vec3::new(-a.cos() * 0.22, 1.0, -a.sin() * 0.22),
    )
}

// Indices des bascules : 0-7 = calques d'objets, 8-12 = effets (suffixe _B).
const HERBE: usize = 0;
const ARBRES: usize = 1;
const POUDRE: usize = 2;
const DECOR: usize = 3;
const ROCHERS: usize = 4;
const ESTRADE: usize = 5;
const COLLINES: usize = 6;
const CIEL: usize = 7;
const PROFONDEUR_B: usize = 8;
const HALO_B: usize = 9;
const ETALONNAGE_B: usize = 10;
const OMBRES_B: usize = 11;
const VENT_B: usize = 12;
const LIBELLES: [&str; 13] = [
    "Herbe & fleurs", "Arbres", "Poudre", "Cloture, fanions & ballons", "Rochers & buissons",
    "Estrade & canons", "Collines", "Ciel", "Profondeur de champ", "Halo lumineux",
    "Vignette & etalonnage", "Ombres", "Vent dans l'herbe",
];
// KeyCode : touche physique du clavier.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/input/keyboard_input.rs#L13-L26
// Doc : https://docs.rs/bevy/latest/bevy/input/keyboard/enum.KeyCode.html
const TOUCHES: [KeyCode; 13] = [
    KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4, KeyCode::F5, KeyCode::F6, KeyCode::F7,
    KeyCode::F8, KeyCode::F9, KeyCode::F10, KeyCode::F11, KeyCode::F12, KeyCode::KeyK,
];
const NOMS_TOUCHES: [&str; 13] = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "K"];

/// État des bascules (donnée globale unique -> ressource).
// Book : https://bevy.org/learn/book/storing-data/resources/
// Doc : https://docs.rs/bevy/latest/bevy/ecs/resource/trait.Resource.html
#[derive(Resource)]
struct Bascules {
    actif: [bool; 13],
    panneau: bool,
}

// Default : valeur utilisée par `init_resource`.
impl Default for Bascules {
    fn default() -> Self {
        Self { actif: [true; 13], panneau: true }
    }
}

/// Indice de la bascule qui montre/masque l'entité.
// Component : donnée attachée à une entité.
// Default + Clone : requis pour utiliser le composant dans `bsn!` (gabarit FromTemplate automatique).
// Book : https://bevy.org/learn/book/storing-data/entities-components/#defining-components
// Doc : https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/template/trait.FromTemplate.html
#[derive(Component, Default, Clone)]
struct Calque(usize);

/// Ligne du panneau (et son texte) associée à la bascule n.
#[derive(Component, Default, Clone)]
struct BoutonBascule(usize);

/// Marqueur du panneau, pour le retrouver dans une requête.
#[derive(Component, Default, Clone)]
struct PanneauBascules;

#[derive(Component, Default, Clone)]
struct Soleil;

#[derive(Component, Default, Clone)]
struct Ballon {
    phase: f32,
    y_base: f32,
}

/// Handle du matériau d'herbe, gardé pour le modifier à chaque image.
// Book : https://bevy.org/learn/book/assets/lifetimes/#preloading
#[derive(Resource)]
struct PoigneeHerbe(Handle<MateriauHerbe>);

/// Caméra orbitale minimale : glisser pour tourner, maj+glisser pour déplacer, molette pour zoomer.
#[derive(Component, Default, Clone)]
struct Orbite {
    distance: f32,
    lacet: f32,
    tangage: f32,
    cible: Vec3,
}

fn main() {
    // App : construit l'application (plugins, ressources, systèmes) puis lance la boucle.
    // Book : https://bevy.org/learn/book/intro/apps-worlds/#apps
    // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html
    App::new()
        // DefaultPlugins : fenêtre, rendu, entrées, assets, UI, scènes...
        // Book : https://bevy.org/learn/book/modular-architecture/plugins/#plugin-groups
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_plugins
        // Doc : https://docs.rs/bevy/latest/bevy/prelude/struct.DefaultPlugins.html
        .add_plugins(DefaultPlugins)
        // Un MaterialPlugin par type de matériau personnalisé : enregistre l'asset et son pipeline.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_material.rs#L12
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MaterialPlugin.html
        .add_plugins((
            MaterialPlugin::<VolumeHoli>::default(),
            MaterialPlugin::<CielHoli>::default(),
            MaterialPlugin::<MateriauNature>::default(),
            MaterialPlugin::<MateriauHerbe>::default(),
        ))
        // Ressource avec une valeur donnée. ClearColor : couleur de fond par défaut des caméras.
        // Book : https://bevy.org/learn/book/storing-data/resources/#initializing-resources
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/window/clear_color.rs#L9
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.insert_resource
        // Doc : https://docs.rs/bevy/latest/bevy/camera/struct.ClearColor.html
        // Doc : https://docs.rs/bevy/latest/bevy/color/enum.Color.html#method.srgb
        .insert_resource(ClearColor(Color::srgb(0.6, 0.75, 0.95)))
        // Lumière ambiante appliquée à toute la scène ; `..default()` complète les autres champs.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/motion_blur.rs#L60-L64
        // Doc : https://docs.rs/bevy/latest/bevy/light/struct.GlobalAmbientLight.html
        // Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.62, 0.75, 1.0),
            brightness: 900.0,
            ..default()
        })
        // Ressource créée avec `Default::default()`.
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.init_resource
        .init_resource::<Bascules>()
        // Startup : exécuté une fois au lancement ; Update : à chaque image.
        // Book : https://bevy.org/learn/book/the-game-loop/schedules/#the-standard-bevy-schedules
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_systems
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.Startup.html
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.Update.html
        .add_systems(Startup, (creer_volume, creer_monde, creer_camera_et_interface))
        .add_systems(
            Update,
            (
                camera_orbitale,
                mise_au_point,
                agiter_herbe,
                animer_ballons,
                saisie_bascules,
                // after : s'exécute après saisie_bascules ; run_if : seulement si Bascules a changé.
                // Book : https://bevy.org/learn/book/the-game-loop/schedules/#arranging-systems-in-schedules
                // Book : https://bevy.org/learn/book/control-flow/run-conditions/#run-conditions
                // Book : https://bevy.org/learn/book/control-flow/change-detection/#resources
                // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.after
                // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.run_if
                // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/common_conditions/fn.resource_changed.html
                appliquer_bascules.after(saisie_bascules).run_if(resource_changed::<Bascules>),
            ),
        )
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.run
        .run();
}

/// Échantillonneur linéaire avec le même mode d'adressage sur les trois axes.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/asset/repeated_texture.rs#L46-L51
// Doc : https://docs.rs/bevy/latest/bevy/image/enum.ImageSampler.html
// Doc : https://docs.rs/bevy/latest/bevy/image/struct.ImageSamplerDescriptor.html
// Doc : https://docs.rs/bevy/latest/bevy/image/enum.ImageAddressMode.html
// Doc : https://docs.rs/bevy/latest/bevy/image/enum.ImageFilterMode.html
fn echantillonneur(mode: ImageAddressMode) -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        address_mode_w: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    })
}

/// Image 3D (l x h x p) à partir d'octets bruts.
fn image_volume(l: usize, h: usize, p: usize, donnees: Vec<u8>, format: TextureFormat, mode: ImageAddressMode) -> Image {
    // Image::new : taille, dimension (D3 = texture 3D), octets, format des texels, usage.
    // RENDER_WORLD : les octets sont libérés côté CPU une fois envoyés au GPU.
    // Book : https://bevy.org/learn/book/the-renderer/render-pipelines/#extract
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/testbed/3d.rs#L538-L548
    // Doc : https://docs.rs/bevy/latest/bevy/image/struct.Image.html#method.new
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/struct.Extent3d.html
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/enum.TextureDimension.html
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/enum.TextureFormat.html
    // Doc : https://docs.rs/bevy/latest/bevy/asset/struct.RenderAssetUsages.html
    let mut img = Image::new(
        Extent3d { width: l as u32, height: h as u32, depth_or_array_layers: p as u32 },
        TextureDimension::D3,
        donnees,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    // Doc : https://docs.rs/bevy/latest/bevy/image/struct.Image.html#structfield.sampler
    img.sampler = echantillonneur(mode);
    img
}

/// Un maillage et son matériau, montrés ou masqués par la bascule `calque`.
// Fonction de scène : renvoie un morceau de scène BSN réutilisable dans d'autres `bsn!`.
// bsn! { A B C } décrit une entité portant les composants A, B et C.
// Mesh3d : maillage à afficher ; MeshMaterial3d::<M> : son matériau (type M explicite dans bsn!).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/scene/bsn.rs#L38-L60
// Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
// Doc : https://docs.rs/bevy/latest/bevy/scene/trait.Scene.html
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh3d.html
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MeshMaterial3d.html
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
fn piece<M: Material>(maillage: Handle<Mesh>, materiau: Handle<M>, calque: usize) -> impl Scene {
    bsn! { Mesh3d(maillage) MeshMaterial3d::<M>(materiau) Calque(calque) }
}

/// Système Startup : calcule le volume de poudre et crée la boîte de poudre et le dôme de ciel.
// Paramètres de système : Commands (modifications différées du monde),
// ResMut<Assets<T>> (accès en écriture à la collection d'assets de type T).
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html
fn creer_volume(
    mut commands: Commands,
    mut maillages: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut volumes: ResMut<Assets<VolumeHoli>>,
    mut ciels: ResMut<Assets<CielHoli>>,
) {
    let (bouches, visees): (Vec<_>, Vec<_>) = (0..N_CANONS).map(disposition_canon).unzip();
    let vol = poudre::construire(&bouches, &visees, &PALETTE, 7);
    let (nx, ny, nz) = (poudre::NX, poudre::NY, poudre::NZ);
    // add : stocke l'asset et renvoie son Handle.
    // Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
    // Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
    let couleur = images.add(image_volume(nx, ny, nz, vol.couleur, TextureFormat::Rgba8Unorm, ImageAddressMode::ClampToEdge));
    let lumiere = images.add(image_volume(nx, ny, nz, vol.lumiere, TextureFormat::Rg8Unorm, ImageAddressMode::ClampToEdge));
    let mut alea = Alea::nouveau(11);
    let bruit: Vec<u8> = (0..64 * 64 * 64 * 4).map(|_| (alea.suivant_u64() >> 56) as u8).collect();
    let detail = images.add(image_volume(64, 64, 64, bruit, TextureFormat::Rgba8Unorm, ImageAddressMode::Repeat));

    let materiau = volumes.add(VolumeHoli {
        params: ParamsVolume {
            boite_min: BOITE_MIN,
            cellule: CELLULE,
            taille_boite: BOITE,
            echelle_pas: 1.0,
            dir_soleil: dir_soleil(),
            gain_soleil: 1.25,
            gain_ciel: 0.55,
            densite: 9.0,
            quantite_detail: 0.85,
            freq_detail: 0.08,
        },
        couleur,
        lumiere,
        detail,
    });
    let ciel = ciels.add(CielHoli { params: ParamsCiel { dir_soleil: dir_soleil(), gain: 1.0 } });
    // spawn_scene_list(bsn_list![a, b]) : crée une entité par élément (séparés par des virgules).
    // #Nom : ajoute un composant Name. Cuboid / Sphere : primitives converties en Mesh par `add`.
    // NotShadowCaster : l'entité ne projette pas d'ombre.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L13-L37
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/shadow_caster_receiver.rs#L49-L54
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene_list
    // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn_list.html
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/name/struct.Name.html
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cuboid.html
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Sphere.html
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.from_xyz
    // Doc : https://docs.rs/bevy/latest/bevy/light/struct.NotShadowCaster.html
    commands.spawn_scene_list(bsn_list![
        #VolumePoudre piece(maillages.add(Cuboid::new(BOITE.x, BOITE.y, BOITE.z)), materiau, POUDRE)
            Transform::from_xyz(0.0, BOITE.y / 2.0, 0.0) NotShadowCaster,
        #Ciel piece(maillages.add(Sphere::new(700.0)), ciel, CIEL) NotShadowCaster,
    ]);
}

/// Système Startup : prairie, collines, herbe, estrade, canons, arbres, rochers, buissons,
/// clôture, fanions, ballons et soleil.
fn creer_monde(
    mut commands: Commands,
    mut maillages: ResMut<Assets<Mesh>>,
    mut standards: ResMut<Assets<StandardMaterial>>,
    mut mats_nature: ResMut<Assets<MateriauNature>>,
    mut mats_herbe: ResMut<Assets<MateriauHerbe>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut alea = Alea::nouveau(3);
    let prairie = mats_nature.add(nature(0.0, [0.10, 0.30, 0.03], [0.42, 0.66, 0.10], 3.0, 0.4, 1.0));
    let colline = mats_nature.add(nature(0.0, [0.14, 0.36, 0.05], [0.45, 0.66, 0.14], 3.0, 0.4, 0.0));
    let buisson = mats_nature.add(nature(3.0, [0.08, 0.26, 0.04], [0.40, 0.64, 0.12], 5.0, 0.6, 1.0));
    let roche = mats_nature.add(nature(2.0, [0.20, 0.20, 0.20], [0.52, 0.50, 0.47], 2.5, 0.5, 1.0));
    let ecorce = mats_nature.add(nature(4.0, [0.05, 0.035, 0.025], [0.17, 0.13, 0.095], 5.0, 0.35, 1.0));

    // prairie + collines ondulées
    // spawn_scene : crée une entité décrite par bsn!.
    // asset_value(x) : ajoute x comme asset et passe son Handle (pas besoin de ResMut<Assets>).
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L21-L26
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
    // Doc : https://docs.rs/bevy/latest/bevy/asset/fn.asset_value.html
    commands.spawn_scene(bsn! {
        #Prairie Mesh3d(asset_value(Cuboid::new(900.0, 0.2, 900.0))) MeshMaterial3d::<MateriauNature>(prairie)
        Transform::from_xyz(0.0, -0.1, 0.0)
    });
    // Sphere::mesh().ico(n) : icosphère subdivisée n fois (erreur si n trop grand, d'où unwrap).
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ecs/iter_combinations.rs#L44
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.SphereMeshBuilder.html#method.ico
    let maillage_colline = maillages.add(Sphere::new(1.0).mesh().ico(4).unwrap());
    let mut collines = Vec::new();
    for i in 0..14 {
        let a = i as f32 / 14.0 * TAU + alea.entre(-0.2, 0.2);
        let r = alea.entre(30.0, 60.0);
        let d = alea.entre(70.0, 120.0);
        let (centre, rayons) = (Vec3::new(a.cos() * d, -r * 0.78, a.sin() * d), Vec3::new(r * 1.6, r, r * 1.3));
        collines.push((centre, rayons));
        // Transform { ... } dans bsn! : seuls les champs cités changent, les autres gardent
        // leur valeur par défaut (rotation nulle ici).
        // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html
        commands.spawn_scene(bsn! {
            piece(maillage_colline.clone(), colline.clone(), COLLINES) Transform { translation: centre, scale: rayons } NotShadowCaster
        });
    }

    // tapis d'herbe fleuri, agité par le vertex shader du vent
    // ExtendedMaterial : `base` = StandardMaterial (rugosité, réflectance...), `extension` = Herbe.
    // cull_mode: None : les deux faces des brins sont dessinées.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L34-L47
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.ExtendedMaterial.html
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
    let herbe = mats_herbe.add(MateriauHerbe {
        base: StandardMaterial {
            perceptual_roughness: 0.8,
            reflectance: 0.3,
            cull_mode: None,
            ..default()
        },
        extension: Herbe { params: ParamsHerbe { maintenant: 0.0, vent: Vec4::new(0.8, 0.0, 0.6, 0.045) } },
    });
    // Insertion différée d'une ressource depuis un système.
    // Book : https://bevy.org/learn/book/storing-data/resources/#dynamic-resources
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html#method.insert_resource
    commands.insert_resource(PoigneeHerbe(herbe.clone()));
    commands.spawn_scene(bsn! { #Herbe piece(maillages.add(maillage_herbe(4)), herbe, HERBE) NotShadowCaster });

    // estrade + canons
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cylinder.html
    commands.spawn_scene(bsn! {
        piece(maillages.add(Cylinder::new(2.2, 0.16)), standards.add(StandardMaterial {
            base_color: Color::srgb(0.93, 0.86, 0.74),
            perceptual_roughness: 0.75,
            ..default()
        }), ESTRADE)
        Transform::from_xyz(0.0, 0.08, 0.0)
    });
    let canon = maillages.add(Cylinder::new(0.17, 0.75));
    let tambour = maillages.add(Cylinder::new(0.32, 0.36));
    let mat_tambour = standards.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.94, 0.9),
        perceptual_roughness: 0.5,
        ..default()
    });
    for (i, coul) in PALETTE.iter().enumerate() {
        let (pos, visee) = disposition_canon(i);
        let m = vers_monde(pos.x, 0.0, pos.z);
        let peinture = standards.add(StandardMaterial {
            base_color: Color::srgb(coul[0], coul[1], coul[2]),
            perceptual_roughness: 0.3,
            ..default()
        });
        let d = visee.normalize();
        // {expr} dans bsn! : valeur calculée par une expression Rust.
        // Quat::from_rotation_arc(a, b) : rotation qui amène la direction a sur b.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/math/render_primitives.rs#L655-L658
        // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Quat.html#method.from_rotation_arc
        commands.spawn_scene_list(bsn_list![
            piece(tambour.clone(), mat_tambour.clone(), ESTRADE) Transform::from_xyz(m.x, 0.18, m.z),
            piece(canon.clone(), peinture, ESTRADE)
                Transform { translation: {Vec3::new(m.x, 0.36, m.z) + d * 0.3}, rotation: Quat::from_rotation_arc(Vec3::Y, d) },
        ]);
    }

    // arbres générés procéduralement (écorce ramifiée + cartes de feuilles à masque alpha)
    let mut emplacements: Vec<(f32, f32, f32)> =
        (0..9).map(|_| (alea.entre(0.0, TAU), alea.entre(10.5, 13.0), alea.entre(1.0, 1.35))).collect();
    emplacements.extend((0..22).map(|i| (i as f32 / 22.0 * TAU + alea.entre(-0.12, 0.12), alea.entre(15.5, 24.0), alea.entre(1.1, 1.7))));
    emplacements.extend((0..45).map(|_| (alea.entre(0.0, TAU), alea.entre(32.0, 70.0), alea.entre(1.4, 2.4))));
    let retenus: Vec<(Vec3, f32, usize)> = emplacements
        .iter()
        .enumerate()
        .filter(|(_, (a, r, _))| !(*r < 13.5 && (a - 0.4).sin().atan2((a - 0.4).cos()).abs() < 0.9))
        .map(|(i, &(a, r, s))| {
            let (x, z) = (a.cos() * r, a.sin() * r);
            (Vec3::new(x, hauteur_sol(&collines, x, z), z), s * 1.2, i)
        })
        .collect();
    let f = foret(&retenus, 5);
    // Texture 2D ; Rgba8UnormSrgb : couleurs en sRGB (converties en linéaire à la lecture).
    let tex_feuille = Image::new(
        Extent3d { width: 128, height: 128, depth_or_array_layers: 1 },
        TextureDimension::D2,
        texture_feuille(128, 2),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // base_color_texture : texture multipliée par la couleur de base.
    // AlphaMode::Mask(0.5) : pixel jeté si alpha < 0.5 (découpe nette, pas de tri).
    // diffuse_transmission : part de lumière diffuse qui traverse la feuille.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/transparency_3d.rs#L26-L38
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/transmission.rs#L106-L114
    // Doc : https://docs.rs/bevy/latest/bevy/material/enum.AlphaMode.html#variant.Mask
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html#structfield.diffuse_transmission
    commands.spawn_scene_list(bsn_list![
        #Branches piece(maillages.add(f.maillage_ecorce()), ecorce, ARBRES),
        #Feuilles piece(maillages.add(f.maillage_feuilles()), standards.add(StandardMaterial {
            base_color_texture: Some(images.add(tex_feuille)),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            perceptual_roughness: 0.9,
            reflectance: 0.15,
            diffuse_transmission: 0.15,
            ..default()
        }), ARBRES),
    ]);

    // rochers et buissons fleuris (chaque ensemble fusionné en un seul maillage)
    let rochers: Vec<_> = (0..14)
        .map(|_| {
            let (a, r, s) = (alea.entre(0.0, TAU), alea.entre(4.5, 11.5), alea.entre(0.25, 0.7));
            (Vec3::new(a.cos() * r, 0.1 * s, a.sin() * r), Vec3::new(s * 1.3, s * 0.6, s), alea.entre(0.0, TAU))
        })
        .collect();
    commands.spawn_scene(bsn! { piece(maillages.add(maillage_ellipsoides(&rochers)), roche, ROCHERS) });
    let mut buissons = Vec::new();
    for _ in 0..14 {
        let (a, r) = (alea.entre(0.0, TAU), alea.entre(5.0, 12.0));
        let (cx, cz) = (a.cos() * r, a.sin() * r);
        for _ in 0..alea.entier(3, 6) {
            let s = alea.entre(0.3, 0.55);
            buissons.push((
                Vec3::new(cx + alea.entre(-0.5, 0.5), s * 0.6, cz + alea.entre(-0.5, 0.5)),
                Vec3::new(s, s * 0.85, s),
                0.0,
            ));
        }
    }
    commands.spawn_scene(bsn! { piece(maillages.add(maillage_ellipsoides(&buissons)), buisson, ROCHERS) });

    // anneau de clôture en bois
    let bois = standards.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.28, 0.17),
        perceptual_roughness: 0.85,
        ..default()
    });
    let poteaux = 48;
    let r_cloture = 14.0;
    let segment = 2.0 * r_cloture * (PI / poteaux as f32).sin();
    let poteau = maillages.add(Cuboid::new(0.12, 1.0, 0.12));
    let lisse = maillages.add(Cuboid::new(segment + 0.1, 0.07, 0.05));
    for i in 0..poteaux {
        let a = i as f32 / poteaux as f32 * TAU;
        commands.spawn_scene(bsn! { piece(poteau.clone(), bois.clone(), DECOR) Transform::from_xyz(a.cos() * r_cloture, 0.5, a.sin() * r_cloture) });
        let am = (i as f32 + 0.5) / poteaux as f32 * TAU;
        let milieu = Vec3::new(am.cos(), 0.0, am.sin()) * r_cloture * (PI / poteaux as f32).cos();
        for y in [0.45, 0.8] {
            // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Quat.html#method.from_rotation_y
            commands.spawn_scene(bsn! {
                piece(lisse.clone(), bois.clone(), DECOR)
                Transform { translation: Vec3::new(milieu.x, y, milieu.z), rotation: Quat::from_rotation_y(-am + PI / 2.0) }
            });
        }
    }

    // fanions : guirlandes qui pendent entre de grands mâts (hors de l'orbite de la caméra)
    let mat_mat = maillages.add(Cylinder::new(0.05, 3.4));
    let fil = maillages.add(Cylinder::new(0.008, 1.0));
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Triangle3d.html
    let fanion = maillages.add(Triangle3d::new(Vec3::new(-0.13, 0.0, 0.0), Vec3::new(0.13, 0.0, 0.0), Vec3::new(0.0, -0.3, 0.0)));
    let blanc = standards.add(StandardMaterial { base_color: Color::srgb(0.95, 0.95, 0.95), ..default() });
    // double_sided + cull_mode: None : triangle visible et éclairé des deux côtés.
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html#structfield.double_sided
    let mats_fanion: Vec<_> = PALETTE
        .iter()
        .map(|c| {
            standards.add(StandardMaterial {
                base_color: Color::srgb(c[0], c[1], c[2]),
                perceptual_roughness: 0.6,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let (mats, rm) = (10, 12.8);
    let sommets: Vec<Vec3> = (0..mats)
        .map(|i| {
            let a = i as f32 / mats as f32 * TAU + 0.1;
            let p = Vec3::new(a.cos() * rm, 0.0, a.sin() * rm);
            commands.spawn_scene(bsn! { piece(mat_mat.clone(), bois.clone(), DECOR) Transform::from_xyz(p.x, 1.7, p.z) });
            Vec3::new(p.x, 3.35, p.z)
        })
        .collect();
    let mut k = 0;
    for i in 0..mats {
        let (p0, p1) = (sommets[i], sommets[(i + 1) % mats]);
        let creux = |t: f32| p0 + (p1 - p0) * t - Vec3::new(0.0, 0.7 * 4.0 * t * (1.0 - t), 0.0);
        let lacet = (-(p1.z - p0.z)).atan2(p1.x - p0.x);
        let mut prec = p0;
        for j in 1..=10 {
            let cour = creux(j as f32 / 10.0);
            let d = cour - prec;
            // Cylindre unitaire étiré (scale.y) et orienté le long du segment.
            commands.spawn_scene(bsn! {
                piece(fil.clone(), blanc.clone(), DECOR) Transform {
                    translation: {(cour + prec) * 0.5},
                    rotation: Quat::from_rotation_arc(Vec3::Y, d.normalize()),
                    scale: Vec3::new(1.0, d.length(), 1.0),
                }
            });
            prec = cour;
        }
        let nfanions = ((p1 - p0).length() / 0.42) as usize;
        for j in 1..nfanions {
            commands.spawn_scene(bsn! {
                piece(fanion.clone(), mats_fanion[k % mats_fanion.len()].clone(), DECOR) NotShadowCaster
                Transform { translation: creux(j as f32 / nfanions as f32), rotation: Quat::from_rotation_y(lacet) }
            });
            k += 1;
        }
    }

    // ballons au bout de leur fil
    let ballon = maillages.add(Sphere::new(0.26));
    for i in 0..14 {
        let a = i as f32 / 14.0 * TAU + alea.entre(-0.15, 0.15);
        let r = alea.entre(4.2, 9.0);
        let y = alea.entre(1.4, 2.8);
        let c = PALETTE[(i * 3) % PALETTE.len()];
        // clearcoat : couche vernie brillante par-dessus le matériau.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/clearcoat.rs#L104-L106
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html#structfield.clearcoat
        let mat = standards.add(StandardMaterial {
            base_color: Color::srgb(c[0], c[1], c[2]),
            perceptual_roughness: 0.2,
            clearcoat: 0.9,
            ..default()
        });
        let longueur = y / 1.15;
        // Children [ ... ] : entités enfants ; leur Transform est relatif au parent.
        // Book : https://bevy.org/learn/book/storing-data/relations/#adding-children-declaratively
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/scene/bsn.rs#L24-L34
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/hierarchy/struct.Children.html
        commands.spawn_scene(bsn! {
            piece(ballon.clone(), mat, DECOR) Ballon { phase: {alea.entre(0.0, TAU)}, y_base: y }
            Transform { translation: Vec3::new(a.cos() * r, y, a.sin() * r), scale: Vec3::new(1.0, 1.15, 1.0) }
            Children [
                Mesh3d({fil.clone()}) MeshMaterial3d::<StandardMaterial>({blanc.clone()})
                Transform { translation: Vec3::new(0.0, -0.25 - longueur / 2.0, 0.0), scale: Vec3::new(1.0, longueur, 1.0) }
            ]
        });
    }

    // DirectionalLight : lumière à rayons parallèles (soleil), orientée par la rotation du Transform.
    // template_value(x) : remplace entièrement le composant par x (ici une chaîne d'appels de méthodes).
    // looking_at(cible, haut) : oriente -Z local vers la cible.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/lighting.rs#L190-L200
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L33-L36
    // Doc : https://docs.rs/bevy/latest/bevy/light/struct.DirectionalLight.html
    // Doc : https://docs.rs/bevy/latest/bevy/scene/fn.template_value.html
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.looking_at
    commands.spawn_scene(bsn! {
        Soleil DirectionalLight { illuminance: 16000.0, shadow_maps_enabled: true, color: Color::srgb(1.0, 0.88, 0.7) }
        template_value(Transform::from_translation(dir_soleil() * 30.0).looking_at(Vec3::ZERO, Vec3::Y))
    });
}

/// Système Startup : caméra avec sa chaîne de post-traitement, titre et panneau de bascules.
fn creer_camera_et_interface(mut commands: Commands) {
    // Camera3d : caméra perspective. Hdr : rendu en flottant (exigé par Bloom).
    // DepthPrepass : profondeur écrite avant la passe principale (lue par holi.wgsl).
    // Tonemapping::AcesFitted : courbe HDR -> écran de style cinéma.
    // ColorGrading : saturation et température globales. Bloom : halo autour des zones claires.
    // Vignette : assombrit les bords. DepthOfField::Bokeh : flou hors du plan de netteté.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/tonemapping.rs#L290-L303
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_prepass.rs#L50-L55
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/color_grading.rs#L322-L327
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/bloom_3d.rs#L27-L36
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/post_processing.rs#L100-L101
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/depth_of_field.rs#L219-L229
    // Doc : https://docs.rs/bevy/latest/bevy/camera/struct.Camera3d.html
    // Doc : https://docs.rs/bevy/latest/bevy/camera/struct.Hdr.html
    // Doc : https://docs.rs/bevy/latest/bevy/core_pipeline/prepass/struct.DepthPrepass.html
    // Doc : https://docs.rs/bevy/latest/bevy/core_pipeline/tonemapping/enum.Tonemapping.html
    // Doc : https://docs.rs/bevy/latest/bevy/render/view/struct.ColorGrading.html
    // Doc : https://docs.rs/bevy/latest/bevy/render/view/struct.ColorGradingGlobal.html
    // Doc : https://docs.rs/bevy/latest/bevy/post_process/bloom/struct.Bloom.html#associatedconstant.NATURAL
    // Doc : https://docs.rs/bevy/latest/bevy/post_process/effect_stack/struct.Vignette.html
    // Doc : https://docs.rs/bevy/latest/bevy/post_process/dof/struct.DepthOfField.html
    // Doc : https://docs.rs/bevy/latest/bevy/post_process/dof/enum.DepthOfFieldMode.html
    commands.spawn_scene(bsn! {
        Camera3d Hdr DepthPrepass template_value(Tonemapping::AcesFitted) Transform
        ColorGrading { global: ColorGradingGlobal { post_saturation: SATURATION, temperature: TEMPERATURE } }
        // Bloom::NATURAL puis Bloom { intensity } : préréglage, puis seul `intensity` est modifié.
        Bloom::NATURAL Bloom { intensity: HALO } Vignette { intensity: VIGNETTE, radius: 0.85, smoothness: 3.0 }
        DepthOfField { mode: DepthOfFieldMode::Bokeh, focal_distance: 11.5, sensor_height: 0.32, aperture_f_stops: OUVERTURE }
        Orbite { distance: 11.5, lacet: 0.4, tangage: 0.17, cible: Vec3::new(0.0, 1.5, 0.0) }
    });

    // Text : texte d'interface. TextFont::font_size : taille (px). Node : boîte de mise en page
    // (flexbox) ; Absolute = placée par rapport à la fenêtre. BackgroundColor : fond de la boîte.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/scene/bsn.rs#L51-L57
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/camera/camera_orbit.rs#L82-L97
    // Doc : https://docs.rs/bevy/latest/bevy/ui/widget/struct.Text.html
    // Doc : https://docs.rs/bevy/latest/bevy/text/struct.TextFont.html
    // Doc : https://docs.rs/bevy/latest/bevy/text/enum.FontSize.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/struct.Node.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/enum.PositionType.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/fn.px.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/struct.BackgroundColor.html
    // Doc : https://docs.rs/bevy/latest/bevy/color/enum.Color.html#method.srgba
    commands.spawn_scene(bsn! {
        Text("HOLI (Rust Bevy 0.19) - poudre statique\nglisser : orbite, maj+glisser : deplacer, molette : zoom")
        TextFont { font_size: px(16) }
        Node { position_type: PositionType::Absolute, top: px(12), left: px(14) } BackgroundColor(Color::srgba(0.1, 0.1, 0.25, 0.3))
    });
    // Panneau en colonne (FlexDirection::Column) : un titre puis une ligne-bouton par bascule.
    // Button : rend le nœud cliquable (ajoute Interaction). TextColor : couleur du texte.
    // La liste d'enfants accepte un Vec de scènes, construit ici par itération.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/layout/flex_layout.rs#L26-L34
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/widgets/button.rs#L86-L99
    // Doc : https://docs.rs/bevy/latest/bevy/ui/enum.FlexDirection.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/struct.UiRect.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/widget/struct.Button.html
    // Doc : https://docs.rs/bevy/latest/bevy/text/struct.TextColor.html
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.SceneList.html
    commands.spawn_scene(bsn! {
        PanneauBascules BackgroundColor(Color::srgba(0.05, 0.08, 0.18, 0.45))
        Node {
            position_type: PositionType::Absolute, bottom: px(12), right: px(12),
            flex_direction: FlexDirection::Column, row_gap: px(2), padding: UiRect::all(px(6)),
        }
        Children [
            Text("bascules (Tab masque)") TextFont { font_size: px(13) } TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
            {(0..LIBELLES.len()).map(|i| bsn! {
                // Node avant Button : garde FocusPolicy::Pass (ordre d'écriture = ordre de résolution).
                Node { padding: UiRect::axes(px(8), px(3)) } Button BoutonBascule(i) BackgroundColor(Color::srgba(0.2, 0.6, 0.3, 0.55))
                Children [Text("") TextFont { font_size: px(14) } BoutonBascule(i)]
            }).collect::<Vec<_>>()}
        ]
    });
}

/// Système Update : rotation, déplacement et zoom de la caméra à la souris, plus une lente dérive.
// Single : exactement une entité correspond (sinon le système est sauté).
// ButtonInput<T> : état des boutons/touches (pressed = maintenu, just_pressed = cette image).
// AccumulatedMouseMotion/Scroll : déplacement de la souris et de la molette cumulés sur l'image.
// Time : delta_secs = durée de l'image (rend la dérive indépendante du nombre d'images/s).
// Book : https://bevy.org/learn/book/storing-data/queries/#working-with-singleton-entities
// Book : https://bevy.org/learn/book/the-game-loop/game-time/#frame-rate-independence-and-delta-time
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/input/mouse_input.rs#L15-L43
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/camera/camera_orbit.rs#L116-L123
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Single.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.Res.html
// Doc : https://docs.rs/bevy/latest/bevy/input/struct.ButtonInput.html
// Doc : https://docs.rs/bevy/latest/bevy/input/mouse/enum.MouseButton.html
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
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.from_translation
    *tr = Transform::from_translation(oeil).looking_at(o.cible, Vec3::Y);
}

/// Système Update : plan de netteté sur la cible ; flou désactivé par une très grande ouverture.
fn mise_au_point(camera: Single<(&Orbite, &mut DepthOfField)>, bascules: Res<Bascules>) {
    let (o, mut pdc) = camera.into_inner();
    pdc.focal_distance = o.distance;
    pdc.aperture_f_stops = if bascules.actif[PROFONDEUR_B] { OUVERTURE } else { 1000.0 };
}

/// Système Update : transmet le temps et la force du vent au matériau d'herbe.
// get_mut : modifie l'asset lui-même (tous ses utilisateurs voient le changement).
// Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#mutating-handles-vs-mutating-assets
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/animated_material.rs#L47-L59
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.get_mut
// Doc : https://docs.rs/bevy/latest/bevy/time/struct.Time.html#method.elapsed_secs
fn agiter_herbe(temps: Res<Time>, bascules: Res<Bascules>, poignee: Res<PoigneeHerbe>, mut mats_herbe: ResMut<Assets<MateriauHerbe>>) {
    if let Some(mut m) = mats_herbe.get_mut(&poignee.0) {
        m.extension.params.maintenant = temps.elapsed_secs();
        m.extension.params.vent.w = if bascules.actif[VENT_B] { 0.045 } else { 0.0 };
    }
}

/// Système Update : les ballons montent, descendent et se balancent.
// Query<(&mut A, &B)> : itère sur toutes les entités portant A et B (A modifiable).
// Book : https://bevy.org/learn/book/storing-data/queries/#mutable-and-immutable-query-data
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Quat.html#method.from_rotation_z
fn animer_ballons(temps: Res<Time>, mut ballons: Query<(&mut Transform, &Ballon)>) {
    let t = temps.elapsed_secs();
    for (mut tr, b) in &mut ballons {
        tr.translation.y = b.y_base + 0.12 * (t * 0.9 + b.phase).sin();
        tr.rotation = Quat::from_rotation_z(0.08 * (t * 0.7 + b.phase * 1.3).sin());
    }
}

/// Système Update : touches et clics inversent les bascules ; Tab montre/masque le panneau.
// Changed<Interaction> : seulement les boutons dont l'état (survol, appui) vient de changer.
// With<T> : filtre sur la présence de T sans le lire. Display::None retire le nœud de la mise en page.
// Book : https://bevy.org/learn/book/control-flow/change-detection/#filtering
// Book : https://bevy.org/learn/book/storing-data/queries/#anatomy-of-a-query
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/widgets/button.rs#L24-L46
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/layout/display_and_visibility.rs#L50-L56
// Doc : https://docs.rs/bevy/latest/bevy/input/struct.ButtonInput.html#method.just_pressed
// Doc : https://docs.rs/bevy/latest/bevy/ui/enum.Interaction.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/query/struct.Changed.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/query/struct.With.html
// Doc : https://docs.rs/bevy/latest/bevy/ui/enum.Display.html
fn saisie_bascules(
    touches: Res<ButtonInput<KeyCode>>,
    mut bascules: ResMut<Bascules>,
    boutons: Query<(&Interaction, &BoutonBascule), Changed<Interaction>>,
    mut panneau: Single<&mut Node, With<PanneauBascules>>,
) {
    let mut inversions: Vec<usize> = TOUCHES.iter().enumerate().filter(|(_, k)| touches.just_pressed(**k)).map(|(i, _)| i).collect();
    if touches.just_pressed(KeyCode::KeyF) {
        inversions.push(PROFONDEUR_B);
    }
    for (interaction, bb) in &boutons {
        if *interaction == Interaction::Pressed {
            inversions.push(bb.0);
        }
    }
    // Écrire dans `bascules` (ResMut) le marque comme modifié : appliquer_bascules s'exécutera.
    for i in inversions {
        bascules.actif[i] = !bascules.actif[i];
    }
    if touches.just_pressed(KeyCode::Tab) {
        bascules.panneau = !bascules.panneau;
        panneau.display = if bascules.panneau { Display::Flex } else { Display::None };
    }
}

/// Système Update (seulement quand Bascules change) : applique l'état aux calques, effets et libellés.
// Visibility::Hidden masque l'entité et ses enfants ; Inherited suit le parent.
// Without<Text> : rend `lignes` disjointe de `libelles` (les deux portent BoutonBascule).
// Book : https://bevy.org/learn/book/storing-data/queries/#mutable-and-immutable-query-data
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/layout/display_and_visibility.rs#L60-L72
// Doc : https://docs.rs/bevy/latest/bevy/camera/visibility/enum.Visibility.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/query/struct.Without.html
#[allow(clippy::type_complexity)]
fn appliquer_bascules(
    bascules: Res<Bascules>,
    mut calques: Query<(&Calque, &mut Visibility)>,
    camera: Single<(&mut Bloom, &mut Vignette, &mut ColorGrading)>,
    mut soleil: Single<&mut DirectionalLight, With<Soleil>>,
    mut libelles: Query<(&mut Text, &BoutonBascule)>,
    mut lignes: Query<(&mut BackgroundColor, &BoutonBascule), Without<Text>>,
) {
    let actif = bascules.actif;
    for (calque, mut vis) in &mut calques {
        *vis = if actif[calque.0] { Visibility::Inherited } else { Visibility::Hidden };
    }
    let (mut halo, mut vignette, mut etalonnage) = camera.into_inner();
    halo.intensity = if actif[HALO_B] { HALO } else { 0.0 };
    vignette.intensity = if actif[ETALONNAGE_B] { VIGNETTE } else { 0.0 };
    etalonnage.global.post_saturation = if actif[ETALONNAGE_B] { SATURATION } else { 1.0 };
    etalonnage.global.temperature = if actif[ETALONNAGE_B] { TEMPERATURE } else { 0.0 };
    soleil.shadow_maps_enabled = actif[OMBRES_B];
    for (mut texte, bb) in &mut libelles {
        texte.0 = format!("[{}]  {}   {}", NOMS_TOUCHES[bb.0], if actif[bb.0] { "oui" } else { "non" }, LIBELLES[bb.0]);
    }
    for (mut fond, bb) in &mut lignes {
        fond.0 = if actif[bb.0] { Color::srgba(0.2, 0.6, 0.3, 0.55) } else { Color::srgba(0.35, 0.35, 0.4, 0.45) };
    }
}
