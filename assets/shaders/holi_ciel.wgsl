// Ciel d'été lumineux : bleu saturé, halo chaud du soleil, gros cumulus ensoleillés.

// VertexOutput : sortie du vertex shader par défaut (position pixel, position monde...).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/custom_material.wgsl#L1-L14
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/forward_io.wgsl#L32-L56
#import bevy_pbr::forward_io::VertexOutput
// view : uniformes de la caméra ; globals : temps écoulé...
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/animate_shader.wgsl#L1-L5
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/mesh_view_bindings.wgsl#L10-L41
// Doc : https://docs.rs/bevy/latest/bevy/render/view/struct.ViewUniform.html
// Doc : https://docs.rs/bevy/latest/bevy/render/globals/struct.GlobalsUniform.html
#import bevy_pbr::mesh_view_bindings::{view, globals}

// Même ordre et mêmes types que `ParamsCiel` (materiaux.rs).
struct CielHoli {
    dir_soleil: vec3<f32>,
    gain: f32,
}

// #{MATERIAL_BIND_GROUP} : groupe des ressources du matériau ; binding 0 = #[uniform(0)].
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/custom_material.wgsl#L5-L7
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> materiau: CielHoli;

// Hash without Sine, (c) 2014 David Hoskins, MIT License (https://www.shadertoy.com/view/4djSRW)
fn hachage(p: vec2<f32>) -> f32 {
    // hachage arithmétique : ceux à base de sin() deviennent pixelisés aux grandes coordonnées
    var p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Bruit de valeur 2D, interpolé en douceur (smoothstep).
fn bruit(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hachage(i), hachage(i + vec2(1.0, 0.0)), u.x),
               mix(hachage(i + vec2(0.0, 1.0)), hachage(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Bruit fractal (fBm) : 6 octaves de `bruit`.
fn bruit_fractal(p_entree: vec2<f32>) -> f32 {
    var p = p_entree;
    var s = 0.0;
    var a = 0.5;
    for (var i = 0; i < 6; i++) {
        s += a * bruit(p);
        p = p * 2.03 + vec2(1.7, 9.2);
        a *= 0.5;
    }
    return s;
}

// @fragment fn fragment : point d'entrée attendu par Bevy pour le fragment shader du matériau.
@fragment
fn fragment(entree: VertexOutput) -> @location(0) vec4<f32> {
    // direction de vue : de la caméra vers le point du dôme
    let dir = normalize(entree.world_position.xyz - view.world_position);
    let dir_soleil = normalize(materiau.dir_soleil);
    let h = max(dir.y, 0.0);
    var coul = mix(vec3(0.62, 0.80, 1.0), vec3(0.05, 0.30, 0.95), pow(h, 0.45)) * 1.15;
    if (dir.y < 0.0) {
        coul = vec3(0.55, 0.72, 0.45);
    }
    let mu = max(dot(dir, dir_soleil), 0.0);
    coul += vec3(1.0, 0.85, 0.6) * (pow(mu, 8.0) * 0.45 + pow(mu, 64.0) * 0.8 + pow(mu, 3000.0) * 40.0);

    // cumulus sur un plan haut : densité fBm, éclairés côté soleil, ventres gris
    if (dir.y > 0.01) {
        let uv = dir.xz / (dir.y + 0.08) * 0.9 + vec2(globals.time * 0.006, 0.0);
        let d = bruit_fractal(uv);
        let couverture = smoothstep(0.50, 0.72, d);
        let vers_soleil = normalize(dir_soleil.xz + vec2(1e-4)) * 0.08;
        let ombrage = clamp((d - bruit_fractal(uv + vers_soleil)) * 5.0 + 0.6, 0.0, 1.0);
        let nuage = mix(vec3(0.72, 0.76, 0.85), vec3(1.05, 1.02, 0.97), ombrage)
                  + vec3(1.0, 0.85, 0.6) * pow(mu, 6.0) * 0.4;
        coul = mix(coul, nuage, couverture * smoothstep(0.01, 0.12, dir.y) * 0.95);
    }
    return vec4(coul * materiau.gain, 1.0);
}
