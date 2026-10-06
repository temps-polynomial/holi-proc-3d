//! Matériau des surfaces naturelles : StandardMaterial étendu dont le fragment shader
//! (`holi_nature.wgsl`) dessine prairie, feuillage, roche, buisson fleuri ou écorce.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Paramètres des surfaces naturelles (struct `Nature` de `holi_nature.wgsl`).
// ShaderType : calcule la disposition mémoire GPU (alignements WGSL) de la struct ;
// l'ordre et les types des champs doivent correspondre à la struct WGSL.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material_bindless.rs#L71-L77
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.ShaderType.html
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec4.html
#[derive(ShaderType, Debug, Clone, Default)]
struct ParamsNature {
    genre: f32,
    couleur_a: Vec4,
    couleur_b: Vec4,
    freq: f32,
    relief: f32,
    fleurs: f32,
}

/// Prairie (0), feuillage (1), roche (2), buisson fleuri (3), écorce (4).
// Extension du StandardMaterial : ajoute ses propres bindings ; 100+ pour ne pas
// chevaucher ceux du StandardMaterial (0-99).
// Asset + TypePath : stocké dans une collection d'assets. AsBindGroup : bindings GPU.
// Book : https://bevy.org/learn/book/assets/custom-assets/#defining-an-asset-type
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L74-L79
// Doc : https://docs.rs/bevy/latest/bevy/asset/trait.Asset.html
// Doc : https://docs.rs/bevy/latest/bevy/reflect/trait.TypePath.html
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Nature {
    #[uniform(100)]
    params: ParamsNature,
}

// MaterialExtension : remplace certains shaders du matériau de base ; le chemin est relatif à `assets/`.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L100-L108
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.MaterialExtension.html
// Doc : https://docs.rs/bevy/latest/bevy/shader/enum.ShaderRef.html
impl MaterialExtension for Nature {
    fn fragment_shader() -> ShaderRef {
        "shaders/holi_nature.wgsl".into()
    }
}

// ExtendedMaterial<Base, Extension> : matériau complet = StandardMaterial + extension Nature.
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.ExtendedMaterial.html
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
pub type MateriauNature = ExtendedMaterial<StandardMaterial, Nature>;

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
