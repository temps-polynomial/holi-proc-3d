//! Herbe : ~700k brins et 7000 fleurs en un seul maillage, avec un StandardMaterial étendu
//! dont le vertex shader (`holi_herbe.wgsl`) fait onduler les brins au vent.

use std::f32::consts::{PI, TAU};

use bevy::light::NotShadowCaster;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::alea::Alea;
use crate::bascules::{Bascule, Bascules, piece};
use crate::maillage;

/// Plugin : regroupe ressources, matériaux et systèmes d'un domaine ; ajouté à l'App dans main.rs.
// Book : https://bevy.org/learn/book/modular-architecture/plugins/
// Doc : https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html
pub struct PluginHerbe;

impl Plugin for PluginHerbe {
    fn build(&self, app: &mut App) {
        // MaterialPlugin : enregistre le matériau et son pipeline de rendu.
        // Startup : une fois au lancement ; Update : à chaque image.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L18-L20
        // Book : https://bevy.org/learn/book/the-game-loop/schedules/#the-standard-bevy-schedules
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MaterialPlugin.html
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_systems
        app.add_plugins(MaterialPlugin::<MateriauHerbe>::default())
            .add_systems(Startup, creer_herbe)
            .add_systems(Update, agiter_herbe);
    }
}

/// Paramètres du vent (struct `Herbe` de `holi_herbe.wgsl`).
// ShaderType : calcule la disposition mémoire GPU (alignements WGSL) de la struct ;
// l'ordre et les types des champs doivent correspondre à la struct WGSL.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material_bindless.rs#L71-L77
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.ShaderType.html
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec4.html
#[derive(ShaderType, Debug, Clone)]
struct ParamsHerbe {
    /// Temps écoulé (s), mis à jour à chaque image.
    maintenant: f32,
    /// xyz direction, w force
    vent: Vec4,
}

/// Extension du StandardMaterial : ajoute ses propres bindings ; 100+ pour ne pas chevaucher
/// ceux du StandardMaterial (0-99).
// Asset + TypePath : stocké dans une collection d'assets. AsBindGroup : bindings GPU.
// Book : https://bevy.org/learn/book/assets/custom-assets/#defining-an-asset-type
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L74-L79
// Doc : https://docs.rs/bevy/latest/bevy/asset/trait.Asset.html
// Doc : https://docs.rs/bevy/latest/bevy/reflect/trait.TypePath.html
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct Herbe {
    #[uniform(100)]
    params: ParamsHerbe,
}

// MaterialExtension : remplace certains shaders du matériau de base ; le chemin est relatif à `assets/`.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L100-L108
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.MaterialExtension.html
// Doc : https://docs.rs/bevy/latest/bevy/shader/enum.ShaderRef.html
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

// ExtendedMaterial<Base, Extension> : matériau complet = StandardMaterial + extension Herbe.
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.ExtendedMaterial.html
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
type MateriauHerbe = ExtendedMaterial<StandardMaterial, Herbe>;

/// Handle du matériau d'herbe, gardé pour le modifier à chaque image.
// Ressource : donnée globale unique.
// Book : https://bevy.org/learn/book/assets/lifetimes/#preloading
// Doc : https://docs.rs/bevy/latest/bevy/ecs/resource/trait.Resource.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/enum.Handle.html
#[derive(Resource)]
struct PoigneeHerbe(Handle<MateriauHerbe>);

/// Système Startup : matériau, ressource PoigneeHerbe et entité du tapis d'herbe.
// Commands : modifications différées du monde ; ResMut<Assets<T>> : écriture dans la collection
// d'assets de type T ; add : stocke l'asset et renvoie son Handle.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
fn creer_herbe(mut commands: Commands, mut maillages: ResMut<Assets<Mesh>>, mut mats_herbe: ResMut<Assets<MateriauHerbe>>) {
    // `base` = StandardMaterial (rugosité, réflectance...), `extension` = Herbe.
    // cull_mode: None : les deux faces des brins sont dessinées.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L34-L47
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html#structfield.cull_mode
    // Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
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
    // spawn_scene(bsn! {...}) : crée une entité ; #Nom ajoute un composant Name.
    // piece(...) : maillage + matériau + calque (voir bascules.rs). NotShadowCaster : pas d'ombre projetée.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L21-L26
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
    // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/name/struct.Name.html
    // Doc : https://docs.rs/bevy/latest/bevy/light/struct.NotShadowCaster.html
    commands.spawn_scene(bsn! { #Herbe piece(maillages.add(maillage_herbe(4)), herbe, Bascule::Herbe) NotShadowCaster });
}

/// Système Update : transmet le temps et la force du vent au matériau d'herbe.
// Res<T> : lecture d'une ressource. get_mut : modifie l'asset lui-même (tous ses utilisateurs
// voient le changement). Time::elapsed_secs : secondes depuis le lancement.
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#mutating-handles-vs-mutating-assets
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/animated_material.rs#L47-L59
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.Res.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.get_mut
// Doc : https://docs.rs/bevy/latest/bevy/time/struct.Time.html#method.elapsed_secs
fn agiter_herbe(temps: Res<Time>, bascules: Res<Bascules>, poignee: Res<PoigneeHerbe>, mut mats_herbe: ResMut<Assets<MateriauHerbe>>) {
    if let Some(mut m) = mats_herbe.get_mut(&poignee.0) {
        m.extension.params.maintenant = temps.elapsed_secs();
        m.extension.params.vent.w = if bascules.actif(Bascule::Vent) { 0.045 } else { 0.0 };
    }
}

/// ~700k brins courbes en touffes, plus pâquerettes et boutons d'or, en un seul maillage.
/// uv.y = hauteur le long du brin, lue par le vertex shader du vent.
fn maillage_herbe(graine: u64) -> Mesh {
    let mut alea = Alea::nouveau(graine);
    let touffes = |k: usize, r0: f32, r1: f32, alea: &mut Alea| -> Vec<(f32, f32)> {
        (0..k)
            .map(|_| {
                let r = (alea.entre(r0 * r0, r1 * r1)).sqrt();
                let th = alea.unif() * TAU;
                (r * th.cos(), r * th.sin())
            })
            .collect()
    };
    let interieur = touffes(46_000, 2.35, 14.2, &mut alea);
    let exterieur = touffes(22_000, 14.2, 30.0, &mut alea);

    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    let mut uv = Vec::new();
    let mut coul = Vec::new();
    let mut idx = Vec::new();

    for (groupe, par_touffe) in [(&interieur, 12), (&exterieur, 8)] {
        for &(tx, tz) in groupe.iter() {
            let h_touffe = 0.16 + 0.22 * alea.unif().powf(1.3);
            let c_touffe = alea.unif();
            let plaque = 0.5 + 0.5 * (tx * 0.45 + 1.3).sin() * (tz * 0.37 - 0.4).sin();
            let incl_touffe = (alea.normale() * 0.35, alea.normale() * 0.35);
            for _ in 0..par_touffe {
                let mut bx = tx + alea.normale() * 0.045;
                let mut bz = tz + alea.normale() * 0.045;
                let d = (bx * bx + bz * bz).sqrt();
                if d < 2.3 {
                    bx *= 2.3 / d.max(1e-3);
                    bz *= 2.3 / d.max(1e-3);
                }
                let h = h_touffe * (0.55 + 0.6 * alea.unif());
                let (ox, oz) = (bx - tx, bz - tz);
                let norme_o = (ox * ox + oz * oz).sqrt().max(1e-3);
                let lx = (incl_touffe.0 * 0.6 + ox / norme_o * 0.5 + alea.normale() * 0.2) * h;
                let lz = (incl_touffe.1 * 0.6 + oz / norme_o * 0.5 + alea.normale() * 0.2) * h;
                let ang = lz.atan2(lx) + PI / 2.0 + alea.normale() * 0.4;
                let w = 0.010 + 0.008 * alea.unif();
                let (wx, wz) = (ang.cos() * w, ang.sin() * w);
                let long_incl = (lx * lx + lz * lz).sqrt();
                // paire de sommets (gauche, droite) à la fraction f du brin
                let point = |f: f32, largeur: f32| -> ([f32; 3], [f32; 3]) {
                    let x = bx + lx * f * f;
                    let z = bz + lz * f * f;
                    let y = h * f * (1.0 - 0.25 * f * f * long_incl / h.max(1e-3));
                    (
                        [x - wx * largeur, y, z - wz * largeur],
                        [x + wx * largeur, y, z + wz * largeur],
                    )
                };
                let (b0, b1) = point(0.0, 1.0);
                let (m0, m1) = point(0.5, 0.8);
                let (pointe, _) = point(1.0, 0.0);
                let base = pos.len() as u32;
                pos.extend_from_slice(&[b0, b1, m0, m1, pointe]);
                uv.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [0.0, 0.5], [1.0, 0.5], [0.5, 1.0]]);
                idx.extend_from_slice(&[0, 1, 2, 1, 3, 2, 2, 3, 4].map(|i| base + i));
                // couleur : racines sombres, pointes ensoleillées ; varie par touffe et par plaque
                let c = c_touffe * 0.6 + plaque * 0.4;
                let k = 0.8 + 0.35 * alea.unif();
                let c_pointe = [
                    (0.13 + 0.2 * c) * k,
                    (0.36 + 0.14 * c) * k,
                    (0.035 + 0.03 * (1.0 - c)) * k,
                    1.0,
                ];
                let c_milieu = [c_pointe[0] * 0.55, c_pointe[1] * 0.55, c_pointe[2] * 0.55, 1.0];
                let c_racine = [0.025, 0.08, 0.012, 1.0];
                coul.extend_from_slice(&[c_racine, c_racine, c_milieu, c_milieu, c_pointe]);
                // Vec3 : vecteur 3D (glam) ; méthodes (normalize, to_array...) sur la même page.
                // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
                // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
                let n = Vec3::new(lx / h.max(1e-3) * 0.3, 1.0, lz / h.max(1e-3) * 0.3).normalize();
                for _ in 0..5 {
                    nrm.push(n.to_array());
                }
            }
        }
    }

    // fleurs : pâquerettes à 8 pétales et boutons d'or (même éventail) au-dessus de l'herbe
    for _ in 0..7000 {
        let r = (alea.entre(2.4 * 2.4, 14.0 * 14.0)).sqrt();
        let th = alea.unif() * TAU;
        let (fx, fz) = (r * th.cos(), r * th.sin());
        let fy = 0.12 + 0.2 * alea.unif();
        let paquerette = alea.unif() < 0.55;
        let taille = if paquerette { 0.034 } else { 0.024 } * (0.8 + 0.4 * alea.unif());
        let rot = alea.unif() * TAU;
        let (tx, tz) = (alea.normale() * 0.25, alea.normale() * 0.25);
        let petale = if paquerette { [0.8, 0.8, 0.76, 1.0] } else { [0.95, 0.7, 0.04, 1.0] };
        let centre = if paquerette { [1.0, 0.72, 0.05, 1.0] } else { [0.95, 0.6, 0.03, 1.0] };
        let base = pos.len() as u32;
        pos.push([fx, fy + 0.006, fz]);
        coul.push(centre);
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU + rot;
            let (ox, oz) = (a.cos() * taille, a.sin() * taille);
            pos.push([fx + ox, fy + ox * tx + oz * tz, fz + oz]);
            coul.push(petale);
        }
        for _ in 0..9 {
            nrm.push([0.0, 1.0, 0.0]);
            uv.push([0.5, 0.9]); // les fleurs ondulent comme les pointes des brins
        }
        for k in 1..=8u32 {
            idx.extend_from_slice(&[base, base + k % 8 + 1, base + k]);
        }
    }
    maillage::assembler(pos, nrm, Some(uv), Some(coul), idx)
}
