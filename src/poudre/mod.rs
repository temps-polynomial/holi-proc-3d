//! Poudre colorée : un volume 3D calculé une fois sur le CPU (`volume.rs`), affiché par
//! raymarching dans `holi_poudre.wgsl` à l'intérieur d'une boîte. Les canons visibles sont
//! dans `decor/estrade.rs` ; leur disposition est définie ici.

mod volume;

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;

use crate::PALETTE;
use crate::alea::Alea;
use crate::bascules::{Bascule, piece};
use crate::ciel::dir_soleil;

/// Plugin : regroupe matériaux et systèmes d'un domaine ; ajouté à l'App dans main.rs.
// Book : https://bevy.org/learn/book/modular-architecture/plugins/
// Doc : https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html
pub struct PluginPoudre;

impl Plugin for PluginPoudre {
    fn build(&self, app: &mut App) {
        // MaterialPlugin : enregistre le matériau personnalisé et son pipeline de rendu.
        // Startup : système exécuté une fois au lancement.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_material.rs#L12-L13
        // Book : https://bevy.org/learn/book/the-game-loop/schedules/#the-standard-bevy-schedules
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MaterialPlugin.html
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_systems
        app.add_plugins(MaterialPlugin::<VolumeHoli>::default()).add_systems(Startup, creer_volume);
    }
}

/// Taille d'une cellule du volume de poudre (m).
const CELLULE: f32 = 0.06;
// Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize...) sur la même page.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
const BOITE: Vec3 = Vec3::new(volume::NX as f32 * CELLULE, volume::NY as f32 * CELLULE, volume::NZ as f32 * CELLULE);
const BOITE_MIN: Vec3 = Vec3::new(-BOITE.x / 2.0, 0.0, -BOITE.z / 2.0);
pub const N_CANONS: usize = 8;

/// Coordonnées de cellule du volume -> coordonnées du monde.
pub fn vers_monde(x: f32, y: f32, z: f32) -> Vec3 {
    BOITE_MIN + Vec3::new(x, y, z) * CELLULE
}

/// Position (en cellules) de la bouche du canon i et sa visée (vers l'intérieur et le haut).
pub fn disposition_canon(i: usize) -> (Vec3, Vec3) {
    let a = i as f32 / N_CANONS as f32 * TAU + 0.2;
    let r = 58.0;
    let (cx, cz) = (volume::NX as f32 / 2.0, volume::NZ as f32 / 2.0);
    (
        Vec3::new(cx + a.cos() * r, 5.0, cz + a.sin() * r),
        Vec3::new(-a.cos() * 0.22, 1.0, -a.sin() * 0.22),
    )
}

/// Paramètres du volume, envoyés au GPU en un seul bloc uniforme.
// ShaderType : calcule la disposition mémoire GPU (alignements WGSL) de la struct ;
// l'ordre et les types des champs doivent correspondre à la struct WGSL (`holi_poudre.wgsl`).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material_bindless.rs#L71-L77
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.ShaderType.html
#[derive(ShaderType, Debug, Clone)]
struct ParamsVolume {
    boite_min: Vec3,
    cellule: f32,
    taille_boite: Vec3,
    echelle_pas: f32,
    dir_soleil: Vec3,
    gain_soleil: f32,
    gain_ciel: f32,
    densite: f32,
    quantite_detail: f32,
    freq_detail: f32,
}

/// Poudre en raymarching : textures 3D couleur + lumière, plus un bruit 3D répétable pour le détail.
// Asset : le matériau est stocké dans Assets<VolumeHoli> et référencé par Handle.
// TypePath : nom de type stable, exigé par Asset.
// AsBindGroup : génère le bind group à partir des attributs de champ ;
// chaque numéro = @binding(n) dans le shader (groupe MATERIAL_BIND_GROUP).
// Book : https://bevy.org/learn/book/assets/custom-assets/#defining-an-asset-type
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_material.rs#L42-L51
// Doc : https://docs.rs/bevy/latest/bevy/asset/trait.Asset.html
// Doc : https://docs.rs/bevy/latest/bevy/reflect/trait.TypePath.html
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct VolumeHoli {
    // uniform(0) : struct ParamsVolume au binding 0.
    #[uniform(0)]
    params: ParamsVolume,
    // texture(1, 3d) + sampler(2) : texture 3D et son échantillonneur.
    // Handle<Image> : référence vers une image de Assets<Image>.
    // Doc : https://docs.rs/bevy/latest/bevy/asset/enum.Handle.html
    // Doc : https://docs.rs/bevy/latest/bevy/image/struct.Image.html
    #[texture(1, dimension = "3d")]
    #[sampler(2)]
    couleur: Handle<Image>,
    #[texture(3, dimension = "3d")]
    #[sampler(4)]
    lumiere: Handle<Image>,
    #[texture(5, dimension = "3d")]
    #[sampler(6)]
    detail: Handle<Image>,
}

// Material : décrit les shaders et l'état de rendu ; chaque méthode a une valeur par défaut.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_material.rs#L53-L63
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html
impl Material for VolumeHoli {
    // Fragment shader personnalisé ; le chemin est relatif au dossier `assets/`.
    // Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.fragment_shader
    // Doc : https://docs.rs/bevy/latest/bevy/shader/enum.ShaderRef.html
    fn fragment_shader() -> ShaderRef {
        "shaders/holi_poudre.wgsl".into()
    }
    // Transparent, couleur prémultipliée par l'alpha (le shader renvoie (couleur * opacité, opacité)).
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.alpha_mode
    // Doc : https://docs.rs/bevy/latest/bevy/material/enum.AlphaMode.html
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }
}

/// Système Startup : calcule le volume de poudre et crée la boîte qui l'affiche.
// Paramètres de système : Commands (modifications différées du monde),
// ResMut<Assets<T>> (accès en écriture à la collection d'assets de type T).
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
fn creer_volume(
    mut commands: Commands,
    mut maillages: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut volumes: ResMut<Assets<VolumeHoli>>,
) {
    let (bouches, visees): (Vec<_>, Vec<_>) = (0..N_CANONS).map(disposition_canon).unzip();
    let vol = volume::construire(&bouches, &visees, &PALETTE, 7);
    let (nx, ny, nz) = (volume::NX, volume::NY, volume::NZ);
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
    // spawn_scene(bsn! {...}) : crée une entité ; #Nom ajoute un composant Name.
    // piece(...) : maillage + matériau + calque (voir bascules.rs). Cuboid : primitive convertie
    // en Mesh par `add`. NotShadowCaster : l'entité ne projette pas d'ombre.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L21-L26
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/shadow_caster_receiver.rs#L49-L54
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
    // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/name/struct.Name.html
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cuboid.html
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.from_xyz
    // Doc : https://docs.rs/bevy/latest/bevy/light/struct.NotShadowCaster.html
    commands.spawn_scene(bsn! {
        #VolumePoudre piece(maillages.add(Cuboid::new(BOITE.x, BOITE.y, BOITE.z)), materiau, Bascule::Poudre)
        Transform::from_xyz(0.0, BOITE.y / 2.0, 0.0) NotShadowCaster
    });
}

/// Échantillonneur linéaire avec le même mode d'adressage sur les trois axes.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/asset/repeated_texture.rs#L46-L51
// Doc : https://docs.rs/bevy/latest/bevy/image/enum.ImageSampler.html
// Doc : https://docs.rs/bevy/latest/bevy/image/struct.ImageSamplerDescriptor.html
// Doc : https://docs.rs/bevy/latest/bevy/image/enum.ImageAddressMode.html
// Doc : https://docs.rs/bevy/latest/bevy/image/enum.ImageFilterMode.html
// Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
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
