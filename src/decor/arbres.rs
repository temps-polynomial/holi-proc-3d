//! Arbres feuillus générés : tronc et branches en tubes (écorce), couronne de cartes de
//! feuilles à masque alpha. Toute la forêt tient en deux maillages : écorce et feuilles.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Mesh;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::AleaDecor;
use super::nature::{MateriauNature, nature};
use super::terrain::Collines;
use crate::alea::Alea;
use crate::bascules::{Bascule, piece};
use crate::maillage;

/// Système Startup : choisit les emplacements, fait pousser la forêt et crée ses deux entités.
// Commands : modifications différées du monde. Res<T>/ResMut<T> : lecture/écriture d'une
// ressource (collections d'assets, générateur partagé, collines posées par le système précédent).
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.Res.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
// Doc : https://docs.rs/bevy/latest/bevy/image/struct.Image.html
pub fn creer_arbres(
    mut commands: Commands,
    mut maillages: ResMut<Assets<Mesh>>,
    mut standards: ResMut<Assets<StandardMaterial>>,
    mut mats_nature: ResMut<Assets<MateriauNature>>,
    mut images: ResMut<Assets<Image>>,
    collines: Res<Collines>,
    mut alea: ResMut<AleaDecor>,
) {
    let alea = &mut alea.0;
    // add : stocke l'asset et renvoie son Handle.
    // Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
    // Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
    let ecorce = mats_nature.add(nature(4.0, [0.05, 0.035, 0.025], [0.17, 0.13, 0.095], 5.0, 0.35, 1.0));
    let mut emplacements: Vec<(f32, f32, f32)> =
        (0..9).map(|_| (alea.entre(0.0, TAU), alea.entre(10.5, 13.0), alea.entre(1.0, 1.35))).collect();
    emplacements.extend((0..22).map(|i| (i as f32 / 22.0 * TAU + alea.entre(-0.12, 0.12), alea.entre(15.5, 24.0), alea.entre(1.1, 1.7))));
    emplacements.extend((0..45).map(|_| (alea.entre(0.0, TAU), alea.entre(32.0, 70.0), alea.entre(1.4, 2.4))));
    // Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize, cross, length...) sur la même page.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
    // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
    let retenus: Vec<(Vec3, f32, usize)> = emplacements
        .iter()
        .enumerate()
        .filter(|(_, (a, r, _))| !(*r < 13.5 && (a - 0.4).sin().atan2((a - 0.4).cos()).abs() < 0.9))
        .map(|(i, &(a, r, s))| {
            let (x, z) = (a.cos() * r, a.sin() * r);
            (Vec3::new(x, collines.hauteur_sol(x, z), z), s * 1.2, i)
        })
        .collect();
    let f = foret(&retenus, 5);
    // Image::new : taille, dimension (D2), octets, format des texels, usage.
    // Rgba8UnormSrgb : couleurs en sRGB (converties en linéaire à la lecture).
    // RENDER_WORLD : les octets sont libérés côté CPU une fois envoyés au GPU.
    // Book : https://bevy.org/learn/book/the-renderer/render-pipelines/#extract
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/testbed/3d.rs#L538-L548
    // Doc : https://docs.rs/bevy/latest/bevy/image/struct.Image.html#method.new
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/struct.Extent3d.html
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/enum.TextureDimension.html
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/enum.TextureFormat.html
    // Doc : https://docs.rs/bevy/latest/bevy/asset/struct.RenderAssetUsages.html
    let tex_feuille = Image::new(
        Extent3d { width: 128, height: 128, depth_or_array_layers: 1 },
        TextureDimension::D2,
        texture_feuille(128, 2),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // spawn_scene_list(bsn_list![a, b]) : une entité par élément ; #Nom ajoute un composant Name.
    // piece(...) : maillage + matériau + calque (voir bascules.rs).
    // StandardMaterial { ..default() } : matériau PBR, champs non cités par défaut.
    // base_color_texture : texture multipliée par la couleur de base.
    // AlphaMode::Mask(0.5) : pixel jeté si alpha < 0.5 (découpe nette, pas de tri).
    // cull_mode: None : les deux faces sont dessinées.
    // diffuse_transmission : part de lumière diffuse qui traverse la feuille.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L13-L37
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/transparency_3d.rs#L26-L38
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/transmission.rs#L106-L114
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene_list
    // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn_list.html
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/name/struct.Name.html
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
    // Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
    // Doc : https://docs.rs/bevy/latest/bevy/material/enum.AlphaMode.html#variant.Mask
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html#structfield.diffuse_transmission
    commands.spawn_scene_list(bsn_list![
        #Branches piece(maillages.add(f.maillage_ecorce()), ecorce, Bascule::Arbres),
        #Feuilles piece(maillages.add(f.maillage_feuilles()), standards.add(StandardMaterial {
            base_color_texture: Some(images.add(tex_feuille)),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            perceptual_roughness: 0.9,
            reflectance: 0.15,
            diffuse_transmission: 0.15,
            ..default()
        }), Bascule::Arbres),
    ]);
}

/// Nombre de côtés des tubes d'écorce.
const COTES: usize = 7;

/// Deux axes orthonormés perpendiculaires à `d`.
fn repere(d: Vec3) -> (Vec3, Vec3) {
    let a = if d.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = d.cross(a).normalize();
    (u, d.cross(u))
}

/// Sommets accumulés de toute la forêt : écorce d'un côté, cartes de feuilles de l'autre.
#[derive(Default)]
struct Foret {
    ecorce_pos: Vec<[f32; 3]>,
    ecorce_nrm: Vec<[f32; 3]>,
    ecorce_idx: Vec<u32>,
    feuille_pos: Vec<[f32; 3]>,
    feuille_nrm: Vec<[f32; 3]>,
    feuille_uv: Vec<[f32; 2]>,
    feuille_coul: Vec<[f32; 4]>,
}

impl Foret {
    fn maillage_ecorce(&self) -> Mesh {
        maillage::assembler(self.ecorce_pos.clone(), self.ecorce_nrm.clone(), None, None, self.ecorce_idx.clone())
    }

    fn maillage_feuilles(&self) -> Mesh {
        let cartes = self.feuille_pos.len() as u32 / 4;
        let idx = (0..cartes)
            .flat_map(|c| [0, 1, 2, 0, 2, 3].map(|i| c * 4 + i))
            .collect();
        maillage::assembler(
            self.feuille_pos.clone(),
            self.feuille_nrm.clone(),
            Some(self.feuille_uv.clone()),
            Some(self.feuille_coul.clone()),
            idx,
        )
    }

    /// Tube passant par `points`, de rayon `rayons[i]` en chaque point.
    fn tube(&mut self, points: &[Vec3], rayons: &[f32]) {
        let base = self.ecorce_pos.len() as u32;
        for (i, &p) in points.iter().enumerate() {
            let d = (points[(i + 1).min(points.len() - 1)] - points[i.saturating_sub(1)]).normalize_or(Vec3::Y);
            let (u, v) = repere(d);
            for s in 0..COTES {
                let a = s as f32 / COTES as f32 * TAU;
                let n = u * a.cos() + v * a.sin();
                self.ecorce_pos.push((p + n * rayons[i]).to_array());
                self.ecorce_nrm.push(n.to_array());
            }
        }
        for r in 0..points.len() - 1 {
            for s in 0..COTES {
                let a = (r * COTES + s) as u32 + base;
                let b = (r * COTES + (s + 1) % COTES) as u32 + base;
                let (c, d) = (a + COTES as u32, b + COTES as u32);
                self.ecorce_idx.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }

    /// Touffe de `cartes` cartes de feuilles autour de `c`, dans la couronne (centre, rayons).
    #[allow(clippy::too_many_arguments)]
    fn grappe(
        &mut self,
        alea: &mut Alea,
        c: Vec3,
        rayon: f32,
        centre_couronne: Vec3,
        rayons_couronne: Vec3,
        cartes: usize,
        taille: f32,
        teinte: Vec3,
    ) {
        for _ in 0..cartes {
            let dir = alea.unitaire();
            let r = rayon * (0.55 + 0.45 * alea.unif().sqrt());
            let centre = c + dir * r;
            let face = (dir + alea.normale3() * 0.8).normalize_or(Vec3::Y);
            let haut = face.cross(alea.normale3()).normalize_or(Vec3::X);
            let droite = haut.cross(face);
            let s = taille * (0.75 + 0.5 * alea.unif()) * 0.5;
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                self.feuille_pos.push((centre + (droite * x + haut * y) * s).to_array());
            }
            // normale d'ombrage : sortante de la touffe, mêlée à la sortante de l'ellipsoïde de couronne
            let sortie_c = (centre - c).normalize_or(Vec3::Y);
            let rel = (centre - centre_couronne) / rayons_couronne;
            let sortie_t = (rel / rayons_couronne).normalize_or(Vec3::Y);
            let n = (sortie_c * 0.3 + sortie_t * 0.7).normalize_or(Vec3::Y);
            // occlusion : sombre au cœur et sous la couronne, ensoleillé au sommet
            let coque = rel.length().clamp(0.0, 1.0);
            let locale = ((r / rayon - 0.55) / 0.45).clamp(0.0, 1.0);
            let dessous = (-rel.y).clamp(0.0, 1.0);
            let mut lumiere =
                (0.3 + 0.7 * coque * coque) * (0.75 + 0.25 * locale) * (1.0 - 0.35 * dessous);
            lumiere = lumiere * (0.9 + 0.25 * rel.y.clamp(0.0, 1.0)) + alea.normale() * 0.05;
            let coul = (teinte * lumiere).clamp(Vec3::ZERO, Vec3::ONE);
            for uv in [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]] {
                self.feuille_nrm.push(n.to_array());
                self.feuille_uv.push(uv);
                self.feuille_coul.push([coul.x, coul.y, coul.z, 1.0]);
            }
        }
    }

    /// Branche récursive : un tube de 3 segments, puis 2 sous-branches tant que `profondeur` > 0.
    #[allow(clippy::too_many_arguments)]
    fn branche(&mut self, alea: &mut Alea, pointes: &mut Vec<Vec3>, p0: Vec3, d: Vec3, longueur: f32, r0: f32, profondeur: u32) {
        let (mut points, mut rayons) = (vec![p0], vec![r0]);
        let (mut p, mut dd) = (p0, d);
        for k in 0..3 {
            dd = (dd + alea.normale3() * 0.07 + Vec3::new(0.0, 0.05, 0.0)).normalize();
            p += dd * longueur / 3.0;
            points.push(p);
            rayons.push(r0 * (1.0 - 0.13 * (k + 1) as f32));
        }
        self.tube(&points, &rayons);
        pointes.push(p);
        if profondeur == 0 {
            return;
        }
        for _ in 0..2 {
            let rotation = alea.entre(0.0, TAU);
            let (u, v) = repere(dd);
            let ecart = alea.entre(0.35, 0.65);
            let mut nd = dd * ecart.cos() + (u * rotation.cos() + v * rotation.sin()) * ecart.sin();
            nd.y = nd.y.max(0.25);
            let r_fin = *rayons.last().unwrap();
            let long = longueur * alea.entre(0.6, 0.75);
            self.branche(alea, pointes, p, nd.normalize(), long, r_fin * 0.72, profondeur - 1);
        }
    }

    /// Un feuillu : tronc droit évasé, 3-4 branches montantes et une couronne arrondie
    /// garnie de touffes (plus sombres dedans et dessous, ensoleillées dessus).
    fn pousser(&mut self, alea: &mut Alea, base: Vec3, echelle: f32, teinte: Vec3, densite_feuilles: f32) {
        let mut pointes = Vec::new();
        let h_tronc = echelle * alea.entre(1.15, 1.45);
        let sommet = base + Vec3::new(alea.normale() * 0.04, 1.0, alea.normale() * 0.04) * h_tronc;
        let r0 = 0.2 * echelle;
        self.tube(
            &[
                base - Vec3::new(0.0, 0.05, 0.0),
                base + Vec3::new(0.0, 0.12 * echelle, 0.0),
                base + (sommet - base) * 0.5,
                sommet,
            ],
            &[r0 * 1.7, r0 * 1.15, r0 * 0.95, r0 * 0.8],
        );
        let branches = alea.entier(3, 5);
        let rotation0 = alea.entre(0.0, TAU);
        for i in 0..branches {
            let a = rotation0 + i as f32 / branches as f32 * TAU + alea.normale() * 0.3;
            let ecart = alea.entre(0.5, 0.8);
            let d = Vec3::new(a.cos() * ecart.sin(), ecart.cos(), a.sin() * ecart.sin());
            let long = echelle * alea.entre(0.8, 1.0);
            self.branche(alea, &mut pointes, sommet, d, long, r0 * 0.62, 2);
        }
        let centre = sommet + Vec3::new(0.0, 0.95 * echelle, 0.0);
        let rayons = Vec3::new(1.45, 1.2, 1.45) * echelle
            * Vec3::new(alea.entre(0.9, 1.1), alea.entre(0.9, 1.1), alea.entre(0.9, 1.1));
        let touffes = (34.0 * densite_feuilles) as usize + 6;
        for _ in 0..touffes {
            let dir = alea.unitaire();
            let coque = 0.55 + 0.45 * alea.unif().powf(0.35);
            let proche = pointes[alea.entier(0, pointes.len() as u32) as usize];
            let c = (centre + dir * rayons * coque) * 0.75 + proche * 0.25;
            let cartes = (95.0 * densite_feuilles) as usize;
            let rayon = 0.42 * echelle * alea.entre(0.8, 1.15);
            let t = teinte * alea.entre(0.88, 1.12);
            self.grappe(alea, c, rayon, centre, rayons, cartes, 0.13 * echelle, t);
        }
    }
}

/// emplacements : (position du pied, échelle, indice de teinte).
fn foret(emplacements: &[(Vec3, f32, usize)], graine: u64) -> Foret {
    let teintes = [
        Vec3::new(0.26, 0.52, 0.08),
        Vec3::new(0.32, 0.58, 0.10),
        Vec3::new(0.22, 0.46, 0.09),
    ];
    let mut alea = Alea::nouveau(graine);
    let mut f = Foret::default();
    for &(base, s, k) in emplacements {
        let densite = if (base.x * base.x + base.z * base.z).sqrt() < 30.0 { 1.0 } else { 0.45 };
        f.pousser(&mut alea, base, s, teintes[k % teintes.len()], densite);
    }
    f
}

/// Carte RGBA8 de ~12 petites feuilles pointues ; l'alpha sert de masque.
fn texture_feuille(n: usize, graine: u64) -> Vec<u8> {
    let mut alea = Alea::nouveau(graine);
    let mut rgba = vec![0u8; n * n * 4];
    for _ in 0..12 {
        let (cx, cy) = (alea.entre(0.2, 0.8), alea.entre(0.2, 0.8));
        let a = alea.entre(0.0, TAU);
        let (long, larg) = (alea.entre(0.15, 0.21), alea.entre(0.08, 0.11));
        let variation = alea.entre(-0.08, 0.08);
        for py in 0..n {
            for px in 0..n {
                let x = px as f32 / (n - 1) as f32 - cx;
                let y = py as f32 / (n - 1) as f32 - cy;
                let u = x * a.cos() + y * a.sin();
                let v = -x * a.sin() + y * a.cos();
                let t = (u / long).clamp(-1.0, 1.0);
                let demi_largeur = larg * (1.0 - t * t).max(0.0).powf(0.8) * (1.0 - 0.35 * t);
                if u.abs() < long && v.abs() < demi_largeur {
                    let nervure = if v.abs() < 0.005 { 0.08 } else { 0.0 };
                    let nuance = (0.8 + 0.1 * (v / (demi_largeur + 1e-6)) - nervure + variation).clamp(0.0, 1.0);
                    let i = (py * n + px) * 4;
                    let s = (nuance * 255.0) as u8;
                    rgba[i..i + 4].copy_from_slice(&[s, s, s, 255]);
                }
            }
        }
    }
    rgba
}
