// Surfaces naturelles procédurales par-dessus le pipeline PBR standard.
//   genre 0 : prairie (verts tachetés, plaques ensoleillées, mouchetures de pâquerettes et boutons d'or)
//   genre 1 : feuillage (touffes : normale bosselée, occlusion des creux, variation de couleur)
//   genre 2 : roche (pierre grise mouchetée, bosselée)
//   genre 3 : buisson fleuri (feuillage + fleurs blanches)
//   genre 4 : écorce (grain vertical, sillons sombres)
// Tout est en coordonnées monde : n'importe quel maillage (plan, colline, amas de sphères) convient.

// pbr_input_from_standard_material : remplit un PbrInput à partir du StandardMaterial de base.
// apply_pbr_lighting : calcule l'éclairage PBR. main_pass_post_lighting_processing :
// brouillard, prémultiplication alpha, etc. FragmentOutput : sortie du fragment shader.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/extended_material.wgsl#L31-L65
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/pbr_fragment.wgsl#L75
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/pbr_functions.wgsl#L336
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/pbr_functions.wgsl#L995
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/forward_io.wgsl#L58
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.MaterialExtension.html
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

// Même ordre et mêmes types que `ParamsNature` (decor/nature.rs).
struct Nature {
    genre: f32,
    couleur_a: vec4<f32>,
    couleur_b: vec4<f32>,
    freq: f32,
    relief: f32,
    fleurs: f32,
}

// binding 100 = #[uniform(100)] de l'extension (0-99 : StandardMaterial).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/extended_material.wgsl#L28-L29
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> nature: Nature;

// Hash without Sine, (c) 2014 David Hoskins, MIT License (https://www.shadertoy.com/view/4djSRW)
fn hachage3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.zyx + 31.32);
    return fract((q.x + q.y) * q.z);
}

// Bruit de valeur 3D, interpolé en douceur.
fn bruit_valeur(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(mix(hachage3(i), hachage3(i + vec3(1.0, 0.0, 0.0)), u.x),
                mix(hachage3(i + vec3(0.0, 1.0, 0.0)), hachage3(i + vec3(1.0, 1.0, 0.0)), u.x), u.y);
    let b = mix(mix(hachage3(i + vec3(0.0, 0.0, 1.0)), hachage3(i + vec3(1.0, 0.0, 1.0)), u.x),
                mix(hachage3(i + vec3(0.0, 1.0, 1.0)), hachage3(i + vec3(1.0, 1.0, 1.0)), u.x), u.y);
    return mix(a, b, u.z);
}

// Bruit fractal (fBm) : 4 octaves, normalisé vers [0, 1].
fn bruit_fractal(p_entree: vec3<f32>) -> f32 {
    var p = p_entree;
    var s = 0.0;
    var a = 0.5;
    for (var i = 0; i < 4; i++) {
        s += a * bruit_valeur(p);
        p = p * 2.07 + vec3(3.1, 1.7, 4.3);
        a *= 0.5;
    }
    return s / 0.9375;
}

// Mouchetures sur une grille perturbée : renvoie (masque, tirage) pour des fleurs de taille `cellule`.
fn mouchetures(p: vec2<f32>, cellule: f32, chance: f32) -> vec2<f32> {
    let g = p / cellule;
    let id = floor(g);
    let h = hachage3(vec3(id, 7.0));
    if (h > chance) {
        return vec2(0.0);
    }
    let c = vec2(hachage3(vec3(id, 1.0)), hachage3(vec3(id, 2.0))) * 0.6 + 0.2;
    let d = length(fract(g) - c);
    return vec2(1.0 - smoothstep(0.10, 0.16, d), hachage3(vec3(id, 3.0)));
}

// Bosselage économique : gradient de deux octaves de bruit par différences avant (6 lectures).
fn bosselage(p: vec3<f32>, freq: f32) -> vec3<f32> {
    let q = p * freq;
    let e = 0.3;
    let n = bruit_valeur(q) + 0.5 * bruit_valeur(q * 2.3);
    let nx = bruit_valeur(q + vec3(e, 0.0, 0.0)) + 0.5 * bruit_valeur((q + vec3(e, 0.0, 0.0)) * 2.3);
    let ny = bruit_valeur(q + vec3(0.0, e, 0.0)) + 0.5 * bruit_valeur((q + vec3(0.0, e, 0.0)) * 2.3);
    let nz = bruit_valeur(q + vec3(0.0, 0.0, e)) + 0.5 * bruit_valeur((q + vec3(0.0, 0.0, e)) * 2.3);
    return vec3(nx - n, ny - n, nz - n) / e;
}

@fragment
fn fragment(entree: VertexOutput, @builtin(front_facing) face_avant: bool) -> FragmentOutput {
    let m = nature;
    // pbr : entrées de l'éclairage (pbr.material.base_color, perceptual_roughness, pbr.N = normale,
    // pbr.diffuse_occlusion...), modifiées ci-dessous avant le calcul de la lumière.
    // Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/pbr_types.wgsl#L97
    var pbr = pbr_input_from_standard_material(entree, face_avant);
    let p = entree.world_position.xyz;
    let genre = i32(m.genre + 0.5);
    let a = m.couleur_a.rgb;
    let b = m.couleur_b.rgb;

    if (genre == 0) {
        let grand = bruit_fractal(vec3(p.xz * 0.18, 0.0));
        let moyen = bruit_fractal(vec3(p.xz * 1.3, 5.0));
        let fin = bruit_valeur(vec3(p.xz * 14.0, 9.0));
        var coul = mix(a, b, smoothstep(0.3, 0.75, grand)) * (0.78 + 0.3 * moyen) * (0.88 + 0.2 * fin);
        // fleurs : pâquerettes et boutons d'or, estompées au loin pour ne pas scintiller
        let fondu = 1.0 - smoothstep(18.0, 40.0, length(p.xz));
        let paquerette = mouchetures(p.xz, 0.11, 0.05 * m.fleurs);
        let bouton_or = mouchetures(p.xz + 0.37, 0.09, 0.05 * m.fleurs);
        coul = mix(coul, vec3(0.95, 0.95, 0.9), paquerette.x * fondu);
        coul = mix(coul, vec3(1.0, 0.82, 0.12), bouton_or.x * fondu);
        pbr.material.base_color = vec4(coul, 1.0);
        pbr.material.perceptual_roughness = 0.92;
        pbr.N = normalize(pbr.N + vec3(moyen - 0.5, 0.0, fin - 0.5) * 0.25);
    } else if (genre == 1 || genre == 3) {
        let touffes = bruit_fractal(p * m.freq);
        let fin = bruit_valeur(p * m.freq * 5.0);
        var coul = mix(a, b, smoothstep(0.35, 0.72, touffes)) * (0.8 + 0.3 * fin);
        if (genre == 3) {
            let s = mouchetures(vec2(p.x + p.y * 0.7, p.z - p.y * 0.4), 0.14, 0.18);
            coul = mix(coul, vec3(0.97, 0.96, 0.92), s.x);
        }
        pbr.material.base_color = vec4(coul, 1.0);
        pbr.material.perceptual_roughness = 0.75;
        pbr.N = normalize(pbr.N + bosselage(p, m.freq) * m.relief);
        // les creux du feuillage sont sombres
        pbr.diffuse_occlusion = vec3(mix(0.6, 1.0, smoothstep(0.25, 0.7, touffes)));
    } else if (genre == 4) {
        // écorce : grain vertical (bruit étiré selon y) et sillons plus sombres
        let grain = bruit_fractal(vec3(p.x * m.freq * 3.0, p.y * m.freq * 0.35, p.z * m.freq * 3.0));
        let sillon = smoothstep(0.35, 0.6, grain);
        let coul = mix(a, b, sillon) * (0.85 + 0.2 * bruit_valeur(p * m.freq * 2.0));
        pbr.material.base_color = vec4(coul, 1.0);
        pbr.material.perceptual_roughness = 0.9;
        pbr.N = normalize(pbr.N + bosselage(vec3(p.x * 3.0, p.y * 0.35, p.z * 3.0), m.freq) * m.relief);
        pbr.diffuse_occlusion = vec3(mix(0.6, 1.0, sillon));
    } else {
        let n = bruit_fractal(p * m.freq);
        let fin = bruit_valeur(p * m.freq * 6.0);
        let coul = mix(a, b, n) * (0.85 + 0.25 * fin);
        pbr.material.base_color = vec4(coul, 1.0);
        pbr.material.perceptual_roughness = 0.8;
        pbr.N = normalize(pbr.N + bosselage(p, m.freq) * m.relief);
        pbr.diffuse_occlusion = vec3(mix(0.55, 1.0, n));
    }

    var sortie: FragmentOutput;
    sortie.color = apply_pbr_lighting(pbr);
    sortie.color = main_pass_post_lighting_processing(pbr, sortie.color);
    return sortie;
}
