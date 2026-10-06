//! Calcul du volume de poudre sur le CPU (fait une seule fois au lancement).
//!
//! Produit les deux textures 3D lues par le shader de raymarching (`holi_poudre.wgsl`) :
//!   couleur : RGBA8 = couleur de la poudre, sqrt(densité / 5)
//!   lumiere : RG8   = transmittance du soleil (poudre au-dessus du voxel), occlusion locale
//! Chaque panache est une chaîne de bouffées douces qui monte depuis un canon.

// Vec3 : vecteur 3D (glam) réexporté par Bevy ; opérateurs et méthodes (splat, min, max,
// distance_squared, normalize) sont décrits sur la même page.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
use bevy::math::Vec3;

use crate::alea::Alea;

/// Résolution de la grille (en cellules).
pub const NX: usize = 160;
pub const NY: usize = 128;
pub const NZ: usize = 160;

/// Octets bruts des deux textures 3D.
pub struct Volume {
    pub couleur: Vec<u8>,
    pub lumiere: Vec<u8>,
}

/// Indice linéaire de la cellule (x, y, z), x variant le plus vite.
fn indice(x: usize, y: usize, z: usize) -> usize {
    (z * NY + y) * NX + x
}

/// Ajoute une sphère de poudre à bord doux (unités : cellules).
fn bouffee(densite: &mut [f32], teinte: &mut [[f32; 3]], centre: Vec3, r: f32, quantite: f32, coul: [f32; 3]) {
    let bas = (centre - Vec3::splat(r)).max(Vec3::ZERO);
    let haut = (centre + Vec3::splat(r)).min(Vec3::new(NX as f32 - 1.0, NY as f32 - 1.0, NZ as f32 - 1.0));
    for z in bas.z as usize..=haut.z as usize {
        for y in bas.y as usize..=haut.y as usize {
            for x in bas.x as usize..=haut.x as usize {
                let d2 = Vec3::new(x as f32, y as f32, z as f32).distance_squared(centre) / (r * r);
                if d2 < 1.0 {
                    let w = (1.0 - d2) * (1.0 - d2) * quantite;
                    let i = indice(x, y, z);
                    densite[i] += w;
                    for k in 0..3 {
                        teinte[i][k] += w * coul[k];
                    }
                }
            }
        }
    }
}

/// Panaches partant des `bouches` (coordonnées en cellules), le long des `visees`, de `couleurs`.
pub fn construire(bouches: &[Vec3], visees: &[Vec3], couleurs: &[[f32; 3]], graine: u64) -> Volume {
    let n = NX * NY * NZ;
    let mut densite = vec![0.0f32; n];
    let mut teinte = vec![[0.0f32; 3]; n];
    let mut alea = Alea::nouveau(graine);
    for ((&bouche, &visee), &coul) in bouches.iter().zip(visees).zip(couleurs) {
        let hauteur = alea.entre(45.0, 75.0);
        let mut p = bouche;
        let mut direction = visee.normalize();
        let etapes = 16;
        for s in 0..etapes {
            let t = s as f32 / etapes as f32;
            direction = (direction + alea.normale3() * 0.12 + Vec3::new(0.0, 0.15, 0.0)).normalize();
            p += direction * hauteur / etapes as f32;
            let r = 5.0 + 11.0 * t.sqrt();
            // chaque bouffée est un amas de sous-bouffées : panache bosselé, pas un tube
            for _ in 0..5 {
                let decalage = alea.normale3() * r * 0.35;
                bouffee(&mut densite, &mut teinte, p + decalage, r * alea.entre(0.55, 0.85), 1.1 * (1.0 - 0.4 * t), coul);
            }
        }
    }

    // soleil presque au zénith : transmittance = exp(-k * poudre au-dessus)
    let mut soleil = vec![0.0f32; n];
    for z in 0..NZ {
        for x in 0..NX {
            let mut dessus = 0.0;
            for y in (0..NY).rev() {
                let i = indice(x, y, z);
                soleil[i] = (-0.35 * (dessus + densite[i] * 0.5)).exp();
                dessus += densite[i];
            }
        }
    }
    // occlusion locale : flou boîte séparable de 9 cellules sur la densité
    let flou = flou_boite(&densite, 4);

    let mut couleur = vec![0u8; n * 4];
    let mut lumiere = vec![0u8; n * 2];
    let octet = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    for i in 0..n {
        let inv = 1.0 / (densite[i] + 1e-4);
        couleur[i * 4] = octet(teinte[i][0] * inv);
        couleur[i * 4 + 1] = octet(teinte[i][1] * inv);
        couleur[i * 4 + 2] = octet(teinte[i][2] * inv);
        couleur[i * 4 + 3] = octet((densite[i] / 5.0).sqrt());
        lumiere[i * 2] = octet(soleil[i]);
        lumiere[i * 2 + 1] = octet((-0.9 * flou[i]).exp());
    }
    Volume { couleur, lumiere }
}

/// Flou boîte de rayon `rayon`, appliqué successivement selon x, y puis z.
fn flou_boite(source: &[f32], rayon: usize) -> Vec<f32> {
    let mut a = source.to_vec();
    let mut b = vec![0.0f32; source.len()];
    let axes = [(NX, 1usize), (NY, NX), (NZ, NX * NY)];
    for &(longueur, pas) in &axes {
        for (i, sortie) in b.iter_mut().enumerate() {
            let coord = (i / pas) % longueur;
            let bas = coord.saturating_sub(rayon);
            let haut = (coord + rayon).min(longueur - 1);
            let base = i - coord * pas;
            let mut somme = 0.0;
            for c in bas..=haut {
                somme += a[base + c * pas];
            }
            *sortie = somme / (2 * rayon + 1) as f32;
        }
        std::mem::swap(&mut a, &mut b);
    }
    a
}
