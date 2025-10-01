use std::f64::consts::PI;
use bincode::{Encode, Decode};

#[derive(Clone, Copy, Debug)]
pub struct C {
    pub re: f64,
    pub im: f64,
}

impl C {
    pub fn new(re: f64, im: f64) -> Self {
        C { re, im }
    }

    pub fn abs_sq(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    pub fn add(&self, other: C) -> C {
        C::new(self.re + other.re, self.im + other.im)
    }

    pub fn mul(&self, other: C) -> C {
        C::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re
        )
    }
}

#[derive(Clone, Copy, Debug, Encode, Decode)]
pub struct Param {
    pub p: f64,
    pub theta: f64,
}

impl Param {
    pub fn new(p: f64, theta: f64) -> Self {
        let p = p.clamp(0.05, 0.95);
        let theta = ((theta + PI) % (2.0 * PI)) - PI;
        Param { p, theta }
    }

    pub fn dist(&self, other: &Param) -> f64 {
        let dp = self.p - other.p;
        let mut dth = (self.theta - other.theta).abs();
        if dth > PI { dth = 2.0 * PI - dth; }
        (dp * dp + dth * dth).sqrt()
    }
}

pub fn param_to_c(param: Param) -> C {
    // Map p ∈ [0.05, 0.95] to a radius in a lively annulus for Julia sets.
    //  r ∈ [0.3, 0.9] works well; tweak to taste.
    let r = 0.3 + 0.6 * param.p;
    let (s, c) = param.theta.sin_cos();
    C::new(r * c, r * s)
}