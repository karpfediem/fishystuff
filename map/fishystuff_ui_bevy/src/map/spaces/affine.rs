#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine2D {
    pub a: f64,
    pub b: f64,
    pub tx: f64,
    pub c: f64,
    pub d: f64,
    pub ty: f64,
}

impl Affine2D {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        tx: 0.0,
        c: 0.0,
        d: 1.0,
        ty: 0.0,
    };

    pub const fn new(a: f64, b: f64, tx: f64, c: f64, d: f64, ty: f64) -> Self {
        Self { a, b, tx, c, d, ty }
    }

    pub fn apply(self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.b * y + self.tx,
            self.c * x + self.d * y + self.ty,
        )
    }

    pub fn inverse(self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        let a = self.d * inv_det;
        let b = -self.b * inv_det;
        let c = -self.c * inv_det;
        let d = self.a * inv_det;
        let tx = -(a * self.tx + b * self.ty);
        let ty = -(c * self.tx + d * self.ty);
        Some(Self { a, b, tx, c, d, ty })
    }

    pub fn compose(lhs: Self, rhs: Self) -> Self {
        // compose(lhs, rhs) means: apply rhs first, then lhs.
        Self {
            a: lhs.a * rhs.a + lhs.b * rhs.c,
            b: lhs.a * rhs.b + lhs.b * rhs.d,
            tx: lhs.a * rhs.tx + lhs.b * rhs.ty + lhs.tx,
            c: lhs.c * rhs.a + lhs.d * rhs.c,
            d: lhs.c * rhs.b + lhs.d * rhs.d,
            ty: lhs.c * rhs.tx + lhs.d * rhs.ty + lhs.ty,
        }
    }

    pub fn approx_eq(self, other: Self, eps: f64) -> bool {
        (self.a - other.a).abs() <= eps
            && (self.b - other.b).abs() <= eps
            && (self.tx - other.tx).abs() <= eps
            && (self.c - other.c).abs() <= eps
            && (self.d - other.d).abs() <= eps
            && (self.ty - other.ty).abs() <= eps
    }
}

#[cfg(test)]
mod tests {
    use super::Affine2D;

    #[test]
    fn inverse_roundtrip() {
        let t = Affine2D::new(2.0, 0.5, 10.0, -0.25, 3.0, -2.0);
        let inv = t.inverse().expect("invertible");
        let (x, y) = (123.25, -88.5);
        let (u, v) = t.apply(x, y);
        let (bx, by) = inv.apply(u, v);
        assert!((bx - x).abs() < 1e-9);
        assert!((by - y).abs() < 1e-9);
    }

    #[test]
    fn compose_order() {
        let scale = Affine2D::new(2.0, 0.0, 0.0, 0.0, 3.0, 0.0);
        let shift = Affine2D::new(1.0, 0.0, 5.0, 0.0, 1.0, -7.0);
        let composed = Affine2D::compose(shift, scale);
        let (x, y) = composed.apply(4.0, 6.0);
        assert!((x - 13.0).abs() < 1e-9);
        assert!((y - 11.0).abs() < 1e-9);
    }
}
