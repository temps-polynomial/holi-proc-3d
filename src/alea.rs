//! Petit générateur pseudo-aléatoire déterministe (xorshift64*), suffisant pour disperser le
//! décor. Même graine = même suite de nombres, donc la même scène à chaque lancement.

use std::f32::consts::TAU;

// Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize_or, constante Y) sur la même page.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
use bevy::math::Vec3;

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
