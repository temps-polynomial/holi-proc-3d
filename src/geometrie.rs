//! Géométrie procédurale : touffes d'herbe + fleurs, arbres feuillus à cartes de feuilles,
//! ellipsoïdes fusionnés, texture de feuille.

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
// Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize, normalize_or, cross, length,
// clamp, to_array, constantes ZERO/ONE/X/Y) sont décrits sur la même page.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
use bevy::math::Vec3;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// Petit générateur pseudo-aléatoire déterministe (xorshift64*), suffisant pour disperser le décor.
pub struct Alea(u64);

impl Alea {
    pub fn nouveau(graine: u64) -> Self {
        Self(graine.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn suivant_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniforme dans [0, 1).
    pub fn unif(&mut self) -> f32 {
        (self.suivant_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Uniforme dans [bas, haut).
    pub fn entre(&mut self, bas: f32, haut: f32) -> f32 {
        bas + (haut - bas) * self.unif()
    }
    /// Entier uniforme dans [bas, haut_exclu).
    pub fn entier(&mut self, bas: u32, haut_exclu: u32) -> u32 {
        bas + (self.suivant_u64() % u64::from(haut_exclu - bas)) as u32
    }
    /// Loi normale centrée réduite (Box-Muller).
    pub fn normale(&mut self) -> f32 {
        let u = self.unif().max(1e-7);
        let v = self.unif();
        (-2.0 * u.ln()).sqrt() * (TAU * v).cos()
    }
    pub fn normale3(&mut self) -> Vec3 {
        Vec3::new(self.normale(), self.normale(), self.normale())
    }
    /// Direction aléatoire unitaire.
    pub fn unitaire(&mut self) -> Vec3 {
        self.normale3().normalize_or(Vec3::Y)
    }
}

/// Assemble un maillage de triangles à partir de tableaux de sommets.
fn maillage(
    positions: Vec<[f32; 3]>,
    normales: Vec<[f32; 3]>,
    uvs: Option<Vec<[f32; 2]>>,
    couleurs: Option<Vec<[f32; 4]>>,
    indices: Vec<u32>,
) -> Mesh {
    // Maillage vide ; TriangleList = chaque triplet d'indices forme un triangle.
    // RENDER_WORLD seul : les sommets sont libérés côté CPU après envoi au GPU
    // (recommandé par la doc quand le CPU ne relit pas le maillage).
    // Book : https://bevy.org/learn/book/the-renderer/render-pipelines/#extract
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/generate_custom_mesh.rs#L106-L108
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#method.new
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/enum.PrimitiveTopology.html
    // Doc : https://docs.rs/bevy/latest/bevy/asset/struct.RenderAssetUsages.html
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    // Attributs de sommet : un tableau par attribut, même longueur pour tous.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/generate_custom_mesh.rs#L109-L207
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#method.insert_attribute
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#associatedconstant.ATTRIBUTE_POSITION
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normales);
    if let Some(uv) = uvs {
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }
    if let Some(c) = couleurs {
        // Couleur par sommet, multipliée par la couleur de base du StandardMaterial.
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
    }
    // Indices des triangles, en u32 (plus de 65 535 sommets possibles).
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/generate_custom_mesh.rs#L243
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#method.insert_indices
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/enum.Indices.html
    m.insert_indices(Indices::U32(indices));
    m
}

/// ~700k brins courbes en touffes, plus pâquerettes et boutons d'or, en un seul maillage.
/// uv.y = hauteur le long du brin, lue par le vertex shader du vent.
pub fn maillage_herbe(graine: u64) -> Mesh {
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
    maillage(pos, nrm, Some(uv), Some(coul), idx)
}

/// Plusieurs ellipsoïdes (centre, rayons, lacet) fusionnés en un seul maillage.
pub fn maillage_ellipsoides(ellipsoides: &[(Vec3, Vec3, f32)]) -> Mesh {
    let (lat, lon) = (10usize, 16usize);
    let mut unitaire = Vec::new();
    for i in 0..=lat {
        let t = PI * i as f32 / lat as f32;
        for j in 0..=lon {
            let p = TAU * j as f32 / lon as f32;
            unitaire.push(Vec3::new(t.sin() * p.cos(), t.cos(), t.sin() * p.sin()));
        }
    }
    let mut tri = Vec::new();
    for i in 0..lat {
        for j in 0..lon {
            let a = (i * (lon + 1) + j) as u32;
            let b = ((i + 1) * (lon + 1) + j) as u32;
            tri.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    let (mut pos, mut nrm, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    for (k, &(c, r, lacet)) in ellipsoides.iter().enumerate() {
        let (cy, sy) = (lacet.cos(), lacet.sin());
        let rot = |v: Vec3| Vec3::new(cy * v.x + sy * v.z, v.y, -sy * v.x + cy * v.z);
        for &u in &unitaire {
            pos.push((rot(u * r) + c).to_array());
            nrm.push(rot(u / r).normalize().to_array());
        }
        let decalage = (k * unitaire.len()) as u32;
        idx.extend(tri.iter().map(|i| i + decalage));
    }
    maillage(pos, nrm, None, None, idx)
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
pub struct Foret {
    pub ecorce_pos: Vec<[f32; 3]>,
    pub ecorce_nrm: Vec<[f32; 3]>,
    pub ecorce_idx: Vec<u32>,
    pub feuille_pos: Vec<[f32; 3]>,
    pub feuille_nrm: Vec<[f32; 3]>,
    pub feuille_uv: Vec<[f32; 2]>,
    pub feuille_coul: Vec<[f32; 4]>,
}

impl Foret {
    pub fn maillage_ecorce(&self) -> Mesh {
        maillage(self.ecorce_pos.clone(), self.ecorce_nrm.clone(), None, None, self.ecorce_idx.clone())
    }

    pub fn maillage_feuilles(&self) -> Mesh {
        let cartes = self.feuille_pos.len() as u32 / 4;
        let idx = (0..cartes)
            .flat_map(|c| [0, 1, 2, 0, 2, 3].map(|i| c * 4 + i))
            .collect();
        maillage(
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
    pub fn pousser(&mut self, alea: &mut Alea, base: Vec3, echelle: f32, teinte: Vec3, densite_feuilles: f32) {
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
pub fn foret(emplacements: &[(Vec3, f32, usize)], graine: u64) -> Foret {
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
pub fn texture_feuille(n: usize, graine: u64) -> Vec<u8> {
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
