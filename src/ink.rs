//! The ink pass: wet rows and stains that bloom on the page.

/// The uniforms of `ink.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InkParams {
    pub wet: [f32; 4],
    /// How wet the sketch is, 0 to 1, in x.
    pub sketch: [f32; 4],
    pub stains: [[f32; 4]; MAX_STAINS],
}

pub const MAX_STAINS: usize = 8;
/// How fast a stain soaks out to its size, per second.
const SOAK: f32 = 0.9;

pub const BLUE_BLACK: f32 = 0.0;
pub const OXBLOOD: f32 = 1.0;

#[derive(Clone, Copy)]
struct Stain {
    pos: [f32; 2],
    radius: f32,
    ink: f32,
    born: f32,
}

/// The newest stains; a ninth one replaces the oldest.
#[derive(Default)]
pub struct Stains {
    list: Vec<Stain>,
    next: usize,
}

impl Stains {
    pub fn add(&mut self, pos: [f32; 2], radius: f32, ink: f32, now: f32) {
        let stain = Stain {
            pos,
            radius,
            ink,
            born: now,
        };
        if self.list.len() < MAX_STAINS {
            self.list.push(stain);
        } else {
            self.list[self.next] = stain;
            self.next = (self.next + 1) % MAX_STAINS;
        }
    }

    pub fn uniforms(&self, now: f32) -> [[f32; 4]; MAX_STAINS] {
        let mut out = [[0.0; 4]; MAX_STAINS];
        for (slot, s) in out.iter_mut().zip(&self.list) {
            let soaked = 1.0 - (-(now - s.born).max(0.0) * SOAK).exp();
            *slot = [s.pos[0], s.pos[1], s.radius * soaked.max(0.01), s.ink];
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stains_soak_out_and_the_oldest_gives_way() {
        let mut s = Stains::default();
        s.add([1.0, 2.0], 40.0, OXBLOOD, 0.0);
        assert!(s.uniforms(0.1)[0][2] < s.uniforms(3.0)[0][2]);
        assert!(s.uniforms(30.0)[0][2] <= 40.0);
        for i in 0..MAX_STAINS {
            s.add([i as f32, 9.0], 10.0, BLUE_BLACK, 0.0);
        }
        assert_eq!(s.uniforms(1.0)[0][0], 7.0, "the first stain was replaced");
        assert_eq!(s.list.len(), MAX_STAINS);
    }
}
