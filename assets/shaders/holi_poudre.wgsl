// Nuages de poudre colorée : raymarching des voxels de poudre.
//   tex_couleur : rgb = couleur de la poudre, a = sqrt(densité / 5)
//   tex_lumiere : r = transmittance du soleil (ombre propre), g = occlusion locale
// La marche s'arrête sur la géométrie opaque grâce au prépass de profondeur.

// #import : importe des éléments des modules WGSL de Bevy.
// VertexOutput : sortie du vertex shader par défaut (position pixel, position monde, uv...).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/custom_material.wgsl#L1-L14
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/forward_io.wgsl#L32-L56
#import bevy_pbr::forward_io::VertexOutput
// view : uniformes de la caméra (position, matrices, viewport) ; globals : temps écoulé...
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/animate_shader.wgsl#L1-L5
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/mesh_view_bindings.wgsl#L10-L41
// Doc : https://docs.rs/bevy/latest/bevy/render/view/struct.ViewUniform.html
// Doc : https://docs.rs/bevy/latest/bevy/render/globals/struct.GlobalsUniform.html
#import bevy_pbr::mesh_view_bindings::{view, globals}
// prepass_depth : profondeur écrite par le prépass (composant DepthPrepass de la caméra).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/show_prepass.wgsl#L26-L28
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/prepass/prepass_utils.wgsl#L6
#import bevy_pbr::prepass_utils
// Conversions de repères : pixel -> NDC -> monde.
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/view_transformations.wgsl#L49-L53
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/view_transformations.wgsl#L229-L232
#import bevy_pbr::view_transformations::{frag_coord_to_ndc, position_ndc_to_world}

// Même ordre et mêmes types que `ParamsVolume` (poudre/mod.rs).
struct VolumeHoli {
    boite_min: vec3<f32>,
    cellule: f32,
    taille_boite: vec3<f32>,
    echelle_pas: f32,
    dir_soleil: vec3<f32>,
    gain_soleil: f32,
    gain_ciel: f32,
    densite: f32,
    quantite_detail: f32,
    freq_detail: f32,
}

// #{MATERIAL_BIND_GROUP} : numéro du groupe des ressources du matériau, fourni par Bevy.
// @binding(n) correspond aux attributs #[uniform(n)], #[texture(n)], #[sampler(n)] côté Rust.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/custom_material.wgsl#L5-L7
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> materiau: VolumeHoli;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tex_couleur: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var ech_couleur: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var tex_lumiere: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var ech_lumiere: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var tex_detail: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var ech_detail: sampler;

// Hash without Sine, (c) 2014 David Hoskins, MIT License (https://www.shadertoy.com/view/4djSRW)
fn hachage12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Intersection rayon / boîte (méthode des dalles) : (t d'entrée, t de sortie).
fn impact_boite(orig: vec3<f32>, dir: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> vec2<f32> {
    let inv = 1.0 / dir;
    let t0 = (bmin - orig) * inv;
    let t1 = (bmax - orig) * inv;
    let tmin = min(t0, t1);
    let tmax = max(t0, t1);
    return vec2(max(max(tmin.x, tmin.y), tmin.z), min(min(tmax.x, tmax.y), tmax.z));
}

// Petits tourbillons que la grille ne peut pas résoudre : 3 octaves de bruit de valeur lus
// dans un volume aléatoire répétable, qui ondulent lentement avec le temps.
fn detail(p: vec3<f32>, freq: f32) -> f32 {
    // globals.time : secondes depuis le lancement (revient à 0 toutes les heures).
    let t = globals.time;
    let q = p * freq + vec3(0.0, -t * 0.05, t * 0.02);
    let a = textureSampleLevel(tex_detail, ech_detail, q, 0.0).r;
    let b = textureSampleLevel(tex_detail, ech_detail, q * 2.03 + vec3(0.31, t * 0.03, 0.17), 0.0).g;
    let c = textureSampleLevel(tex_detail, ech_detail, q * 4.11 + vec3(0.73, 0.11, t * 0.05), 0.0).b;
    return (a * 0.62 + b * 0.28 + c * 0.10);
}

// Distance le long du rayon jusqu'à la géométrie opaque déjà dessinée (1e9 si aucune).
fn t_scene(pos: vec4<f32>, orig: vec3<f32>, dir: vec3<f32>) -> f32 {
// DEPTH_PREPASS : défini par Bevy quand la caméra a un DepthPrepass.
#ifdef DEPTH_PREPASS
    let profondeur = prepass_utils::prepass_depth(pos, 0u);
    if (profondeur <= 0.0) {
        return 1e9;
    }
    let monde = position_ndc_to_world(frag_coord_to_ndc(vec4(pos.xy, profondeur, 1.0)));
    return dot(monde - orig, dir);
#else
    return 1e9;
#endif
}

// @fragment fn fragment : point d'entrée attendu par Bevy pour le fragment shader du matériau.
@fragment
fn fragment(entree: VertexOutput) -> @location(0) vec4<f32> {
    let m = materiau;
    let orig = view.world_position;
    let dir = normalize(entree.world_position.xyz - orig);
    let bmin = m.boite_min;
    let impact = impact_boite(orig, dir, bmin, bmin + m.taille_boite);
    let t_debut = max(impact.x, 0.0);
    let t_fin = min(impact.y, t_scene(entree.position, orig, dir));
    if (t_fin <= t_debut) {
        discard;
    }

    let dt = m.cellule * m.echelle_pas;
    let n = min(i32((t_fin - t_debut) / dt) + 1, 600);
    // départ décalé aléatoirement par pixel et par image : remplace les strates par du bruit
    var t = t_debut + dt * hachage12(entree.position.xy + fract(globals.time * 7.13) * 131.0);

    // halo de diffusion vers l'avant quand on regarde vers le soleil
    let mu = dot(dir, normalize(m.dir_soleil));
    let phase = 0.75 + 0.9 * pow(max(mu, 0.0), 6.0);
    let soleil = vec3(1.0, 0.95, 0.85) * m.gain_soleil * phase;
    let ciel = vec3(0.50, 0.68, 1.0) * m.gain_ciel;
    let rebond = vec3(0.30, 0.45, 0.16) * m.gain_ciel;

    var T = 1.0;
    var coul = vec3(0.0);
    for (var i = 0; i < n; i++) {
        if (t > t_fin) {
            break;
        }
        let p = orig + dir * t;
        let uvw = (p - bmin) / m.taille_boite;
        let c = textureSampleLevel(tex_couleur, ech_couleur, uvw, 0.0);
        if (c.a > 0.02) {
            let l = textureSampleLevel(tex_lumiere, ech_lumiere, uvw, 0.0);
            let base = c.a * c.a * 5.0;
            // le détail creuse surtout les bords mous, les cœurs restent pleins
            let bord = 1.0 - smoothstep(0.4, 4.0, base);
            let bruit = detail(p, m.freq_detail);
            // érosion : bruit faible ronge les bords, bruit fort les gonfle
            let bosse = smoothstep(0.32, 0.68, bruit);
            let dens = max(base * (0.35 + 1.3 * bosse) - m.quantite_detail * bord * (1.0 - bosse) * 0.6, 0.0)
                     * m.quantite_detail + base * (1.0 - m.quantite_detail);
            // poudre vive : saturation légèrement poussée
            let luma = dot(c.rgb, vec3(0.299, 0.587, 0.114));
            let albedo = clamp(mix(vec3(luma), c.rgb, 1.25), vec3(0.0), vec3(1.0));
            let hauteur = uvw.y;
            let lumiere = soleil * l.r + ciel * (0.35 + 0.65 * l.g) + rebond * l.g * (1.0 - hauteur);
            let a = exp(-dens * m.densite * dt);
            coul += T * albedo * lumiere * (1.0 - a);
            T *= a;
            if (T < 0.004) {
                break;
            }
        }
        t += dt;
    }
    // couleur déjà multipliée par l'opacité -> AlphaMode::Premultiplied côté Rust
    return vec4(coul, 1.0 - T);
}
