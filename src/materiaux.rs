//! Matériaux personnalisés : volume de poudre, dôme de ciel, surfaces naturelles procédurales
//! et herbe agitée par le vent.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialExtension, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

/// Paramètres du volume, envoyés au GPU en un seul bloc uniforme.
// ShaderType : calcule la disposition mémoire GPU (alignements WGSL) de la struct ;
// l'ordre et les types des champs doivent correspondre à la struct WGSL (`holi.wgsl`).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material_bindless.rs#L71-L77
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.ShaderType.html
#[derive(ShaderType, Debug, Clone)]
pub struct ParamsVolume {
    pub boite_min: Vec3,
    pub cellule: f32,
    pub taille_boite: Vec3,
    pub echelle_pas: f32,
    pub dir_soleil: Vec3,
    pub gain_soleil: f32,
    pub gain_ciel: f32,
    pub densite: f32,
    pub quantite_detail: f32,
    pub freq_detail: f32,
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
pub struct VolumeHoli {
    // uniform(0) : struct ParamsVolume au binding 0.
    #[uniform(0)]
    pub params: ParamsVolume,
    // texture(1, 3d) + sampler(2) : texture 3D et son échantillonneur.
    // Handle<Image> : référence vers une image de Assets<Image>.
    // Doc : https://docs.rs/bevy/latest/bevy/asset/enum.Handle.html
    // Doc : https://docs.rs/bevy/latest/bevy/image/struct.Image.html
    #[texture(1, dimension = "3d")]
    #[sampler(2)]
    pub couleur: Handle<Image>,
    #[texture(3, dimension = "3d")]
    #[sampler(4)]
    pub lumiere: Handle<Image>,
    #[texture(5, dimension = "3d")]
    #[sampler(6)]
    pub detail: Handle<Image>,
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
        "shaders/holi.wgsl".into()
    }
    // Transparent, couleur prémultipliée par l'alpha (le shader renvoie (couleur * opacité, opacité)).
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.alpha_mode
    // Doc : https://docs.rs/bevy/latest/bevy/material/enum.AlphaMode.html
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }
}

/// Paramètres du ciel (struct `CielHoli` de `holi_ciel.wgsl`).
#[derive(ShaderType, Debug, Clone)]
pub struct ParamsCiel {
    pub dir_soleil: Vec3,
    pub gain: f32,
}

/// Dôme de ciel procédural (dégradé, halo solaire, cumulus).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CielHoli {
    #[uniform(0)]
    pub params: ParamsCiel,
}

impl Material for CielHoli {
    fn fragment_shader() -> ShaderRef {
        "shaders/holi_ciel.wgsl".into()
    }
    // false : le matériau n'est pas dessiné dans les cartes d'ombre (ne projette pas d'ombre).
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.enable_shadows
    fn enable_shadows() -> bool {
        false
    }
    // Ajuste le pipeline de rendu généré pour ce matériau.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader_advanced/custom_vertex_attribute.rs#L74-L86
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.specialize
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/struct.RenderPipelineDescriptor.html
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // on regarde le dôme de l'intérieur : on élimine les faces avant au lieu des faces arrière
        // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/enum.Face.html
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

/// Paramètres des surfaces naturelles (struct `Nature` de `holi_nature.wgsl`).
#[derive(ShaderType, Debug, Clone, Default)]
pub struct ParamsNature {
    pub genre: f32,
    pub couleur_a: Vec4,
    pub couleur_b: Vec4,
    pub freq: f32,
    pub relief: f32,
    pub fleurs: f32,
}

/// Prairie (0), feuillage (1), roche (2), buisson fleuri (3), écorce (4).
// Extension du StandardMaterial : ajoute ses propres bindings ; 100+ pour ne pas
// chevaucher ceux du StandardMaterial (0-99).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L74-L79
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Nature {
    #[uniform(100)]
    pub params: ParamsNature,
}

// MaterialExtension : remplace certains shaders du matériau de base.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L100-L108
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.MaterialExtension.html
impl MaterialExtension for Nature {
    fn fragment_shader() -> ShaderRef {
        "shaders/holi_nature.wgsl".into()
    }
}

// ExtendedMaterial<Base, Extension> : matériau complet = StandardMaterial + extension Nature.
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.ExtendedMaterial.html
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
pub type MateriauNature = bevy::pbr::ExtendedMaterial<StandardMaterial, Nature>;

/// Construit un MateriauNature : `genre` (voir `Nature`), couleurs a/b, fréquence du bruit,
/// intensité du relief, présence de fleurs.
pub fn nature(genre: f32, a: [f32; 3], b: [f32; 3], freq: f32, relief: f32, fleurs: f32) -> MateriauNature {
    MateriauNature {
        // StandardMaterial par défaut : fournit l'éclairage PBR, le shader de Nature
        // ne fait que modifier ses entrées.
        base: StandardMaterial::default(),
        extension: Nature {
            params: ParamsNature {
                genre,
                couleur_a: Vec4::new(a[0], a[1], a[2], 1.0),
                couleur_b: Vec4::new(b[0], b[1], b[2], 1.0),
                freq,
                relief,
                fleurs,
            },
        },
    }
}

/// Paramètres du vent (struct `Herbe` de `holi_herbe.wgsl`).
#[derive(ShaderType, Debug, Clone)]
pub struct ParamsHerbe {
    /// Temps écoulé (s), mis à jour à chaque image.
    pub maintenant: f32,
    /// xyz direction, w force
    pub vent: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Herbe {
    #[uniform(100)]
    pub params: ParamsHerbe,
}

impl MaterialExtension for Herbe {
    // Vertex shader : déplace les sommets (le fragment reste celui du StandardMaterial).
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.MaterialExtension.html#method.vertex_shader
    fn vertex_shader() -> ShaderRef {
        "shaders/holi_herbe.wgsl".into()
    }
    // Même déplacement pour le prépass (profondeur) et les ombres, sinon ils verraient l'herbe immobile.
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.MaterialExtension.html#method.prepass_vertex_shader
    fn prepass_vertex_shader() -> ShaderRef {
        "shaders/holi_herbe.wgsl".into()
    }
}

pub type MateriauHerbe = bevy::pbr::ExtendedMaterial<StandardMaterial, Herbe>;
