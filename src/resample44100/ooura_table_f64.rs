#![allow(unused_assignments, unused_parens, unused_variables)]

pub trait Trig64 {
    fn sin(&mut self, input: f64) -> f64;
    fn cos(&mut self, input: f64) -> f64;
}

pub struct RealDft {
    n: usize,
    ip: Vec<i32>,
    w: Vec<f64>,
}

impl RealDft {
    pub fn new(n: usize) -> Result<Self, &'static str> {
        if n < 4 || n > 65536 || !n.is_power_of_two() {
            return Err("candidate supports power-of-two RDFT sizes4..65536");
        }
        Ok(Self { n, ip: vec![0; n+2], w: vec![0.0; n/2] })
    }

    pub fn transform<M: Trig64>(&mut self, a: &mut [f64], positive: bool,
                                math: &mut M) -> Result<(), &'static str> {
        if a.len() != self.n { return Err("fixed RDFT plan length mismatch"); }
        rdft(self.n as i32, if positive { 1 } else { -1 }, a,
             &mut self.ip, &mut self.w, math);
        Ok(())
    }

}

fn rdft<M: Trig64>(n: i32, isgn: i32, a: &mut [f64], ip: &mut [i32], w: &mut [f64], math: &mut M) {
    let mut nw: i32 = 0i32;
    let mut nc: i32 = 0i32;
    let mut xi: f64 = 0f64;
    nw = ip[(0i32) as usize];
    if (n > (nw << 2i32)) {
        nw = (n >> 2i32);
        makewt(nw, ip, w, math);
    }
    nc = ip[(1i32) as usize];
    if (n > (nc << 2i32)) {
        nc = (n >> 2i32);
        makect(nc, ip, &mut w[(nw) as usize..], math);
    }
    if (isgn >= 0i32) {
        if (n > 4i32) {
            bitrv2(n, &mut ip[(2i32) as usize..], a, math);
            cftfsub(n, a, w, math);
            rftfsub(n, a, nc, &mut w[(nw) as usize..], math);
        } else {
            if (n == 4i32) {
                cftfsub(n, a, w, math);
            }
        }
        xi = (a[(0i32) as usize] - a[(1i32) as usize]);
        a[(0i32) as usize] = (a[(0i32) as usize] + a[(1i32) as usize]);
        a[(1i32) as usize] = xi;
    } else {
        a[(1i32) as usize] = (0.5f64 * (a[(0i32) as usize] - a[(1i32) as usize]));
        a[(0i32) as usize] = (a[(0i32) as usize] - a[(1i32) as usize]);
        if (n > 4i32) {
            rftbsub(n, a, nc, &mut w[(nw) as usize..], math);
            bitrv2(n, &mut ip[(2i32) as usize..], a, math);
            cftbsub(n, a, w, math);
        } else {
            if (n == 4i32) {
                cftfsub(n, a, w, math);
            }
        }
    }
}

fn makewt<M: Trig64>(nw: i32, ip: &mut [i32], w: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut nwh: i32 = 0i32;
    let mut delta: f64 = 0f64;
    let mut x: f64 = 0f64;
    let mut y: f64 = 0f64;
    ip[(0i32) as usize] = nw;
    ip[(1i32) as usize] = 1i32;
    if (nw > 2i32) {
        nwh = (nw >> 1i32);
        delta = (f64::from_bits(0x3fe921fb54442d18) / (nwh as f64));
        w[(0i32) as usize] = (1i32 as f64);
        w[(1i32) as usize] = (0i32 as f64);
        w[(nwh) as usize] = math.cos((delta * (nwh as f64)));
        w[((nwh + 1i32)) as usize] = w[(nwh) as usize];
        if (nwh > 2i32) {
            {
                j = 2i32;
                while (j < nwh) {
                    x = math.cos((delta * (j as f64)));
                    y = math.sin((delta * (j as f64)));
                    w[(j) as usize] = x;
                    w[((j + 1i32)) as usize] = y;
                    w[((nw - j)) as usize] = y;
                    w[(((nw - j) + 1i32)) as usize] = x;
                    j = (j + 2i32);
                }
            }
            bitrv2(nw, &mut ip[(2i32) as usize..], w, math);
        }
    }
}

fn makect<M: Trig64>(nc: i32, ip: &mut [i32], c: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut nch: i32 = 0i32;
    let mut delta: f64 = 0f64;
    ip[(1i32) as usize] = nc;
    if (nc > 1i32) {
        nch = (nc >> 1i32);
        delta = (f64::from_bits(0x3fe921fb54442d18) / (nch as f64));
        c[(0i32) as usize] = math.cos((delta * (nch as f64)));
        c[(nch) as usize] = (0.5f64 * c[(0i32) as usize]);
        {
            j = 1i32;
            while (j < nch) {
                c[(j) as usize] = (0.5f64 * math.cos((delta * (j as f64))));
                c[((nc - j)) as usize] = (0.5f64 * math.sin((delta * (j as f64))));
                j += 1;
            }
        }
    }
}

fn bitrv2<M: Trig64>(n: i32, ip: &mut [i32], a: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut k1: i32 = 0i32;
    let mut l: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut m2: i32 = 0i32;
    let mut xr: f64 = 0f64;
    let mut xi: f64 = 0f64;
    let mut yr: f64 = 0f64;
    let mut yi: f64 = 0f64;
    ip[(0i32) as usize] = 0i32;
    l = n;
    m = 1i32;
    while ((m << 3i32) < l) {
        l = (l >> 1i32);
        {
            j = 0i32;
            while (j < m) {
                ip[((m + j)) as usize] = (ip[(j) as usize] + l);
                j += 1;
            }
        }
        m = (m << 1i32);
    }
    m2 = (2i32 * m);
    if ((m << 3i32) == l) {
        {
            k = 0i32;
            while (k < m) {
                {
                    j = 0i32;
                    while (j < k) {
                        j1 = ((2i32 * j) + ip[(k) as usize]);
                        k1 = ((2i32 * k) + ip[(j) as usize]);
                        xr = a[(j1) as usize];
                        xi = a[((j1 + 1i32)) as usize];
                        yr = a[(k1) as usize];
                        yi = a[((k1 + 1i32)) as usize];
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m2);
                        k1 = (k1 + (2i32 * m2));
                        xr = a[(j1) as usize];
                        xi = a[((j1 + 1i32)) as usize];
                        yr = a[(k1) as usize];
                        yi = a[((k1 + 1i32)) as usize];
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m2);
                        k1 = (k1 - m2);
                        xr = a[(j1) as usize];
                        xi = a[((j1 + 1i32)) as usize];
                        yr = a[(k1) as usize];
                        yi = a[((k1 + 1i32)) as usize];
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m2);
                        k1 = (k1 + (2i32 * m2));
                        xr = a[(j1) as usize];
                        xi = a[((j1 + 1i32)) as usize];
                        yr = a[(k1) as usize];
                        yi = a[((k1 + 1i32)) as usize];
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j += 1;
                    }
                }
                j1 = (((2i32 * k) + m2) + ip[(k) as usize]);
                k1 = (j1 + m2);
                xr = a[(j1) as usize];
                xi = a[((j1 + 1i32)) as usize];
                yr = a[(k1) as usize];
                yi = a[((k1 + 1i32)) as usize];
                a[(j1) as usize] = yr;
                a[((j1 + 1i32)) as usize] = yi;
                a[(k1) as usize] = xr;
                a[((k1 + 1i32)) as usize] = xi;
                k += 1;
            }
        }
    } else {
        {
            k = 1i32;
            while (k < m) {
                {
                    j = 0i32;
                    while (j < k) {
                        j1 = ((2i32 * j) + ip[(k) as usize]);
                        k1 = ((2i32 * k) + ip[(j) as usize]);
                        xr = a[(j1) as usize];
                        xi = a[((j1 + 1i32)) as usize];
                        yr = a[(k1) as usize];
                        yi = a[((k1 + 1i32)) as usize];
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j1 = (j1 + m2);
                        k1 = (k1 + m2);
                        xr = a[(j1) as usize];
                        xi = a[((j1 + 1i32)) as usize];
                        yr = a[(k1) as usize];
                        yi = a[((k1 + 1i32)) as usize];
                        a[(j1) as usize] = yr;
                        a[((j1 + 1i32)) as usize] = yi;
                        a[(k1) as usize] = xr;
                        a[((k1 + 1i32)) as usize] = xi;
                        j += 1;
                    }
                }
                k += 1;
            }
        }
    }
}

fn cftfsub<M: Trig64>(n: i32, a: &mut [f64], w: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut j2: i32 = 0i32;
    let mut j3: i32 = 0i32;
    let mut l: i32 = 0i32;
    let mut x0r: f64 = 0f64;
    let mut x0i: f64 = 0f64;
    let mut x1r: f64 = 0f64;
    let mut x1i: f64 = 0f64;
    let mut x2r: f64 = 0f64;
    let mut x2i: f64 = 0f64;
    let mut x3r: f64 = 0f64;
    let mut x3i: f64 = 0f64;
    l = 2i32;
    if (n > 8i32) {
        cft1st(n, a, w, math);
        l = 8i32;
        while ((l << 2i32) < n) {
            cftmdl(n, l, a, w, math);
            l = (l << 2i32);
        }
    }
    if ((l << 2i32) == n) {
        {
            j = 0i32;
            while (j < l) {
                j1 = (j + l);
                j2 = (j1 + l);
                j3 = (j2 + l);
                x0r = (a[(j) as usize] + a[(j1) as usize]);
                x0i = (a[((j + 1i32)) as usize] + a[((j1 + 1i32)) as usize]);
                x1r = (a[(j) as usize] - a[(j1) as usize]);
                x1i = (a[((j + 1i32)) as usize] - a[((j1 + 1i32)) as usize]);
                x2r = (a[(j2) as usize] + a[(j3) as usize]);
                x2i = (a[((j2 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
                x3r = (a[(j2) as usize] - a[(j3) as usize]);
                x3i = (a[((j2 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
                a[(j) as usize] = (x0r + x2r);
                a[((j + 1i32)) as usize] = (x0i + x2i);
                a[(j2) as usize] = (x0r - x2r);
                a[((j2 + 1i32)) as usize] = (x0i - x2i);
                a[(j1) as usize] = (x1r - x3i);
                a[((j1 + 1i32)) as usize] = (x1i + x3r);
                a[(j3) as usize] = (x1r + x3i);
                a[((j3 + 1i32)) as usize] = (x1i - x3r);
                j = (j + 2i32);
            }
        }
    } else {
        {
            j = 0i32;
            while (j < l) {
                j1 = (j + l);
                x0r = (a[(j) as usize] - a[(j1) as usize]);
                x0i = (a[((j + 1i32)) as usize] - a[((j1 + 1i32)) as usize]);
                a[(j) as usize] = (a[(j) as usize] + a[(j1) as usize]);
                a[((j + 1i32)) as usize] = (a[((j + 1i32)) as usize] + a[((j1 + 1i32)) as usize]);
                a[(j1) as usize] = x0r;
                a[((j1 + 1i32)) as usize] = x0i;
                j = (j + 2i32);
            }
        }
    }
}

fn cftbsub<M: Trig64>(n: i32, a: &mut [f64], w: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut j2: i32 = 0i32;
    let mut j3: i32 = 0i32;
    let mut l: i32 = 0i32;
    let mut x0r: f64 = 0f64;
    let mut x0i: f64 = 0f64;
    let mut x1r: f64 = 0f64;
    let mut x1i: f64 = 0f64;
    let mut x2r: f64 = 0f64;
    let mut x2i: f64 = 0f64;
    let mut x3r: f64 = 0f64;
    let mut x3i: f64 = 0f64;
    l = 2i32;
    if (n > 8i32) {
        cft1st(n, a, w, math);
        l = 8i32;
        while ((l << 2i32) < n) {
            cftmdl(n, l, a, w, math);
            l = (l << 2i32);
        }
    }
    if ((l << 2i32) == n) {
        {
            j = 0i32;
            while (j < l) {
                j1 = (j + l);
                j2 = (j1 + l);
                j3 = (j2 + l);
                x0r = (a[(j) as usize] + a[(j1) as usize]);
                x0i = ((-a[((j + 1i32)) as usize]) - a[((j1 + 1i32)) as usize]);
                x1r = (a[(j) as usize] - a[(j1) as usize]);
                x1i = ((-a[((j + 1i32)) as usize]) + a[((j1 + 1i32)) as usize]);
                x2r = (a[(j2) as usize] + a[(j3) as usize]);
                x2i = (a[((j2 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
                x3r = (a[(j2) as usize] - a[(j3) as usize]);
                x3i = (a[((j2 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
                a[(j) as usize] = (x0r + x2r);
                a[((j + 1i32)) as usize] = (x0i - x2i);
                a[(j2) as usize] = (x0r - x2r);
                a[((j2 + 1i32)) as usize] = (x0i + x2i);
                a[(j1) as usize] = (x1r - x3i);
                a[((j1 + 1i32)) as usize] = (x1i - x3r);
                a[(j3) as usize] = (x1r + x3i);
                a[((j3 + 1i32)) as usize] = (x1i + x3r);
                j = (j + 2i32);
            }
        }
    } else {
        {
            j = 0i32;
            while (j < l) {
                j1 = (j + l);
                x0r = (a[(j) as usize] - a[(j1) as usize]);
                x0i = ((-a[((j + 1i32)) as usize]) + a[((j1 + 1i32)) as usize]);
                a[(j) as usize] = (a[(j) as usize] + a[(j1) as usize]);
                a[((j + 1i32)) as usize] = ((-a[((j + 1i32)) as usize]) - a[((j1 + 1i32)) as usize]);
                a[(j1) as usize] = x0r;
                a[((j1 + 1i32)) as usize] = x0i;
                j = (j + 2i32);
            }
        }
    }
}

fn cft1st<M: Trig64>(n: i32, a: &mut [f64], w: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut k1: i32 = 0i32;
    let mut k2: i32 = 0i32;
    let mut wk1r: f64 = 0f64;
    let mut wk1i: f64 = 0f64;
    let mut wk2r: f64 = 0f64;
    let mut wk2i: f64 = 0f64;
    let mut wk3r: f64 = 0f64;
    let mut wk3i: f64 = 0f64;
    let mut x0r: f64 = 0f64;
    let mut x0i: f64 = 0f64;
    let mut x1r: f64 = 0f64;
    let mut x1i: f64 = 0f64;
    let mut x2r: f64 = 0f64;
    let mut x2i: f64 = 0f64;
    let mut x3r: f64 = 0f64;
    let mut x3i: f64 = 0f64;
    x0r = (a[(0i32) as usize] + a[(2i32) as usize]);
    x0i = (a[(1i32) as usize] + a[(3i32) as usize]);
    x1r = (a[(0i32) as usize] - a[(2i32) as usize]);
    x1i = (a[(1i32) as usize] - a[(3i32) as usize]);
    x2r = (a[(4i32) as usize] + a[(6i32) as usize]);
    x2i = (a[(5i32) as usize] + a[(7i32) as usize]);
    x3r = (a[(4i32) as usize] - a[(6i32) as usize]);
    x3i = (a[(5i32) as usize] - a[(7i32) as usize]);
    a[(0i32) as usize] = (x0r + x2r);
    a[(1i32) as usize] = (x0i + x2i);
    a[(4i32) as usize] = (x0r - x2r);
    a[(5i32) as usize] = (x0i - x2i);
    a[(2i32) as usize] = (x1r - x3i);
    a[(3i32) as usize] = (x1i + x3r);
    a[(6i32) as usize] = (x1r + x3i);
    a[(7i32) as usize] = (x1i - x3r);
    wk1r = w[(2i32) as usize];
    x0r = (a[(8i32) as usize] + a[(10i32) as usize]);
    x0i = (a[(9i32) as usize] + a[(11i32) as usize]);
    x1r = (a[(8i32) as usize] - a[(10i32) as usize]);
    x1i = (a[(9i32) as usize] - a[(11i32) as usize]);
    x2r = (a[(12i32) as usize] + a[(14i32) as usize]);
    x2i = (a[(13i32) as usize] + a[(15i32) as usize]);
    x3r = (a[(12i32) as usize] - a[(14i32) as usize]);
    x3i = (a[(13i32) as usize] - a[(15i32) as usize]);
    a[(8i32) as usize] = (x0r + x2r);
    a[(9i32) as usize] = (x0i + x2i);
    a[(12i32) as usize] = (x2i - x0i);
    a[(13i32) as usize] = (x0r - x2r);
    x0r = (x1r - x3i);
    x0i = (x1i + x3r);
    a[(10i32) as usize] = (wk1r * (x0r - x0i));
    a[(11i32) as usize] = (wk1r * (x0r + x0i));
    x0r = (x3i + x1r);
    x0i = (x3r - x1i);
    a[(14i32) as usize] = (wk1r * (x0i - x0r));
    a[(15i32) as usize] = (wk1r * (x0i + x0r));
    k1 = 0i32;
    {
        j = 16i32;
        while (j < n) {
            k1 = (k1 + 2i32);
            k2 = (2i32 * k1);
            wk2r = w[(k1) as usize];
            wk2i = w[((k1 + 1i32)) as usize];
            wk1r = w[(k2) as usize];
            wk1i = w[((k2 + 1i32)) as usize];
            wk3r = (wk1r - (((2i32 as f64) * wk2i) * wk1i));
            wk3i = ((((2i32 as f64) * wk2i) * wk1r) - wk1i);
            x0r = (a[(j) as usize] + a[((j + 2i32)) as usize]);
            x0i = (a[((j + 1i32)) as usize] + a[((j + 3i32)) as usize]);
            x1r = (a[(j) as usize] - a[((j + 2i32)) as usize]);
            x1i = (a[((j + 1i32)) as usize] - a[((j + 3i32)) as usize]);
            x2r = (a[((j + 4i32)) as usize] + a[((j + 6i32)) as usize]);
            x2i = (a[((j + 5i32)) as usize] + a[((j + 7i32)) as usize]);
            x3r = (a[((j + 4i32)) as usize] - a[((j + 6i32)) as usize]);
            x3i = (a[((j + 5i32)) as usize] - a[((j + 7i32)) as usize]);
            a[(j) as usize] = (x0r + x2r);
            a[((j + 1i32)) as usize] = (x0i + x2i);
            x0r = (x0r - x2r);
            x0i = (x0i - x2i);
            a[((j + 4i32)) as usize] = ((wk2r * x0r) - (wk2i * x0i));
            a[((j + 5i32)) as usize] = ((wk2r * x0i) + (wk2i * x0r));
            x0r = (x1r - x3i);
            x0i = (x1i + x3r);
            a[((j + 2i32)) as usize] = ((wk1r * x0r) - (wk1i * x0i));
            a[((j + 3i32)) as usize] = ((wk1r * x0i) + (wk1i * x0r));
            x0r = (x1r + x3i);
            x0i = (x1i - x3r);
            a[((j + 6i32)) as usize] = ((wk3r * x0r) - (wk3i * x0i));
            a[((j + 7i32)) as usize] = ((wk3r * x0i) + (wk3i * x0r));
            wk1r = w[((k2 + 2i32)) as usize];
            wk1i = w[((k2 + 3i32)) as usize];
            wk3r = (wk1r - (((2i32 as f64) * wk2r) * wk1i));
            wk3i = ((((2i32 as f64) * wk2r) * wk1r) - wk1i);
            x0r = (a[((j + 8i32)) as usize] + a[((j + 10i32)) as usize]);
            x0i = (a[((j + 9i32)) as usize] + a[((j + 11i32)) as usize]);
            x1r = (a[((j + 8i32)) as usize] - a[((j + 10i32)) as usize]);
            x1i = (a[((j + 9i32)) as usize] - a[((j + 11i32)) as usize]);
            x2r = (a[((j + 12i32)) as usize] + a[((j + 14i32)) as usize]);
            x2i = (a[((j + 13i32)) as usize] + a[((j + 15i32)) as usize]);
            x3r = (a[((j + 12i32)) as usize] - a[((j + 14i32)) as usize]);
            x3i = (a[((j + 13i32)) as usize] - a[((j + 15i32)) as usize]);
            a[((j + 8i32)) as usize] = (x0r + x2r);
            a[((j + 9i32)) as usize] = (x0i + x2i);
            x0r = (x0r - x2r);
            x0i = (x0i - x2i);
            a[((j + 12i32)) as usize] = (((-wk2i) * x0r) - (wk2r * x0i));
            a[((j + 13i32)) as usize] = (((-wk2i) * x0i) + (wk2r * x0r));
            x0r = (x1r - x3i);
            x0i = (x1i + x3r);
            a[((j + 10i32)) as usize] = ((wk1r * x0r) - (wk1i * x0i));
            a[((j + 11i32)) as usize] = ((wk1r * x0i) + (wk1i * x0r));
            x0r = (x1r + x3i);
            x0i = (x1i - x3r);
            a[((j + 14i32)) as usize] = ((wk3r * x0r) - (wk3i * x0i));
            a[((j + 15i32)) as usize] = ((wk3r * x0i) + (wk3i * x0r));
            j = (j + 16i32);
        }
    }
}

fn cftmdl<M: Trig64>(n: i32, l: i32, a: &mut [f64], w: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut j1: i32 = 0i32;
    let mut j2: i32 = 0i32;
    let mut j3: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut k1: i32 = 0i32;
    let mut k2: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut m2: i32 = 0i32;
    let mut wk1r: f64 = 0f64;
    let mut wk1i: f64 = 0f64;
    let mut wk2r: f64 = 0f64;
    let mut wk2i: f64 = 0f64;
    let mut wk3r: f64 = 0f64;
    let mut wk3i: f64 = 0f64;
    let mut x0r: f64 = 0f64;
    let mut x0i: f64 = 0f64;
    let mut x1r: f64 = 0f64;
    let mut x1i: f64 = 0f64;
    let mut x2r: f64 = 0f64;
    let mut x2i: f64 = 0f64;
    let mut x3r: f64 = 0f64;
    let mut x3i: f64 = 0f64;
    m = (l << 2i32);
    {
        j = 0i32;
        while (j < l) {
            j1 = (j + l);
            j2 = (j1 + l);
            j3 = (j2 + l);
            x0r = (a[(j) as usize] + a[(j1) as usize]);
            x0i = (a[((j + 1i32)) as usize] + a[((j1 + 1i32)) as usize]);
            x1r = (a[(j) as usize] - a[(j1) as usize]);
            x1i = (a[((j + 1i32)) as usize] - a[((j1 + 1i32)) as usize]);
            x2r = (a[(j2) as usize] + a[(j3) as usize]);
            x2i = (a[((j2 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
            x3r = (a[(j2) as usize] - a[(j3) as usize]);
            x3i = (a[((j2 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
            a[(j) as usize] = (x0r + x2r);
            a[((j + 1i32)) as usize] = (x0i + x2i);
            a[(j2) as usize] = (x0r - x2r);
            a[((j2 + 1i32)) as usize] = (x0i - x2i);
            a[(j1) as usize] = (x1r - x3i);
            a[((j1 + 1i32)) as usize] = (x1i + x3r);
            a[(j3) as usize] = (x1r + x3i);
            a[((j3 + 1i32)) as usize] = (x1i - x3r);
            j = (j + 2i32);
        }
    }
    wk1r = w[(2i32) as usize];
    {
        j = m;
        while (j < (l + m)) {
            j1 = (j + l);
            j2 = (j1 + l);
            j3 = (j2 + l);
            x0r = (a[(j) as usize] + a[(j1) as usize]);
            x0i = (a[((j + 1i32)) as usize] + a[((j1 + 1i32)) as usize]);
            x1r = (a[(j) as usize] - a[(j1) as usize]);
            x1i = (a[((j + 1i32)) as usize] - a[((j1 + 1i32)) as usize]);
            x2r = (a[(j2) as usize] + a[(j3) as usize]);
            x2i = (a[((j2 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
            x3r = (a[(j2) as usize] - a[(j3) as usize]);
            x3i = (a[((j2 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
            a[(j) as usize] = (x0r + x2r);
            a[((j + 1i32)) as usize] = (x0i + x2i);
            a[(j2) as usize] = (x2i - x0i);
            a[((j2 + 1i32)) as usize] = (x0r - x2r);
            x0r = (x1r - x3i);
            x0i = (x1i + x3r);
            a[(j1) as usize] = (wk1r * (x0r - x0i));
            a[((j1 + 1i32)) as usize] = (wk1r * (x0r + x0i));
            x0r = (x3i + x1r);
            x0i = (x3r - x1i);
            a[(j3) as usize] = (wk1r * (x0i - x0r));
            a[((j3 + 1i32)) as usize] = (wk1r * (x0i + x0r));
            j = (j + 2i32);
        }
    }
    k1 = 0i32;
    m2 = (2i32 * m);
    {
        k = m2;
        while (k < n) {
            k1 = (k1 + 2i32);
            k2 = (2i32 * k1);
            wk2r = w[(k1) as usize];
            wk2i = w[((k1 + 1i32)) as usize];
            wk1r = w[(k2) as usize];
            wk1i = w[((k2 + 1i32)) as usize];
            wk3r = (wk1r - (((2i32 as f64) * wk2i) * wk1i));
            wk3i = ((((2i32 as f64) * wk2i) * wk1r) - wk1i);
            {
                j = k;
                while (j < (l + k)) {
                    j1 = (j + l);
                    j2 = (j1 + l);
                    j3 = (j2 + l);
                    x0r = (a[(j) as usize] + a[(j1) as usize]);
                    x0i = (a[((j + 1i32)) as usize] + a[((j1 + 1i32)) as usize]);
                    x1r = (a[(j) as usize] - a[(j1) as usize]);
                    x1i = (a[((j + 1i32)) as usize] - a[((j1 + 1i32)) as usize]);
                    x2r = (a[(j2) as usize] + a[(j3) as usize]);
                    x2i = (a[((j2 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
                    x3r = (a[(j2) as usize] - a[(j3) as usize]);
                    x3i = (a[((j2 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
                    a[(j) as usize] = (x0r + x2r);
                    a[((j + 1i32)) as usize] = (x0i + x2i);
                    x0r = (x0r - x2r);
                    x0i = (x0i - x2i);
                    a[(j2) as usize] = ((wk2r * x0r) - (wk2i * x0i));
                    a[((j2 + 1i32)) as usize] = ((wk2r * x0i) + (wk2i * x0r));
                    x0r = (x1r - x3i);
                    x0i = (x1i + x3r);
                    a[(j1) as usize] = ((wk1r * x0r) - (wk1i * x0i));
                    a[((j1 + 1i32)) as usize] = ((wk1r * x0i) + (wk1i * x0r));
                    x0r = (x1r + x3i);
                    x0i = (x1i - x3r);
                    a[(j3) as usize] = ((wk3r * x0r) - (wk3i * x0i));
                    a[((j3 + 1i32)) as usize] = ((wk3r * x0i) + (wk3i * x0r));
                    j = (j + 2i32);
                }
            }
            wk1r = w[((k2 + 2i32)) as usize];
            wk1i = w[((k2 + 3i32)) as usize];
            wk3r = (wk1r - (((2i32 as f64) * wk2r) * wk1i));
            wk3i = ((((2i32 as f64) * wk2r) * wk1r) - wk1i);
            {
                j = (k + m);
                while (j < (l + (k + m))) {
                    j1 = (j + l);
                    j2 = (j1 + l);
                    j3 = (j2 + l);
                    x0r = (a[(j) as usize] + a[(j1) as usize]);
                    x0i = (a[((j + 1i32)) as usize] + a[((j1 + 1i32)) as usize]);
                    x1r = (a[(j) as usize] - a[(j1) as usize]);
                    x1i = (a[((j + 1i32)) as usize] - a[((j1 + 1i32)) as usize]);
                    x2r = (a[(j2) as usize] + a[(j3) as usize]);
                    x2i = (a[((j2 + 1i32)) as usize] + a[((j3 + 1i32)) as usize]);
                    x3r = (a[(j2) as usize] - a[(j3) as usize]);
                    x3i = (a[((j2 + 1i32)) as usize] - a[((j3 + 1i32)) as usize]);
                    a[(j) as usize] = (x0r + x2r);
                    a[((j + 1i32)) as usize] = (x0i + x2i);
                    x0r = (x0r - x2r);
                    x0i = (x0i - x2i);
                    a[(j2) as usize] = (((-wk2i) * x0r) - (wk2r * x0i));
                    a[((j2 + 1i32)) as usize] = (((-wk2i) * x0i) + (wk2r * x0r));
                    x0r = (x1r - x3i);
                    x0i = (x1i + x3r);
                    a[(j1) as usize] = ((wk1r * x0r) - (wk1i * x0i));
                    a[((j1 + 1i32)) as usize] = ((wk1r * x0i) + (wk1i * x0r));
                    x0r = (x1r + x3i);
                    x0i = (x1i - x3r);
                    a[(j3) as usize] = ((wk3r * x0r) - (wk3i * x0i));
                    a[((j3 + 1i32)) as usize] = ((wk3r * x0i) + (wk3i * x0r));
                    j = (j + 2i32);
                }
            }
            k = (k + m2);
        }
    }
}

fn rftfsub<M: Trig64>(n: i32, a: &mut [f64], nc: i32, c: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut kk: i32 = 0i32;
    let mut ks: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut wkr: f64 = 0f64;
    let mut wki: f64 = 0f64;
    let mut xr: f64 = 0f64;
    let mut xi: f64 = 0f64;
    let mut yr: f64 = 0f64;
    let mut yi: f64 = 0f64;
    m = (n >> 1i32);
    ks = ((2i32 * nc) / m);
    kk = 0i32;
    {
        j = 2i32;
        while (j < m) {
            k = (n - j);
            kk = (kk + ks);
            wkr = (0.5f64 - c[((nc - kk)) as usize]);
            wki = c[(kk) as usize];
            xr = (a[(j) as usize] - a[(k) as usize]);
            xi = (a[((j + 1i32)) as usize] + a[((k + 1i32)) as usize]);
            yr = ((wkr * xr) - (wki * xi));
            yi = ((wkr * xi) + (wki * xr));
            a[(j) as usize] = (a[(j) as usize] - yr);
            a[((j + 1i32)) as usize] = (a[((j + 1i32)) as usize] - yi);
            a[(k) as usize] = (a[(k) as usize] + yr);
            a[((k + 1i32)) as usize] = (a[((k + 1i32)) as usize] - yi);
            j = (j + 2i32);
        }
    }
}

fn rftbsub<M: Trig64>(n: i32, a: &mut [f64], nc: i32, c: &mut [f64], math: &mut M) {
    let mut j: i32 = 0i32;
    let mut k: i32 = 0i32;
    let mut kk: i32 = 0i32;
    let mut ks: i32 = 0i32;
    let mut m: i32 = 0i32;
    let mut wkr: f64 = 0f64;
    let mut wki: f64 = 0f64;
    let mut xr: f64 = 0f64;
    let mut xi: f64 = 0f64;
    let mut yr: f64 = 0f64;
    let mut yi: f64 = 0f64;
    a[(1i32) as usize] = (-a[(1i32) as usize]);
    m = (n >> 1i32);
    ks = ((2i32 * nc) / m);
    kk = 0i32;
    {
        j = 2i32;
        while (j < m) {
            k = (n - j);
            kk = (kk + ks);
            wkr = (0.5f64 - c[((nc - kk)) as usize]);
            wki = c[(kk) as usize];
            xr = (a[(j) as usize] - a[(k) as usize]);
            xi = (a[((j + 1i32)) as usize] + a[((k + 1i32)) as usize]);
            yr = ((wkr * xr) + (wki * xi));
            yi = ((wkr * xi) - (wki * xr));
            a[(j) as usize] = (a[(j) as usize] - yr);
            a[((j + 1i32)) as usize] = (yi - a[((j + 1i32)) as usize]);
            a[(k) as usize] = (a[(k) as usize] + yr);
            a[((k + 1i32)) as usize] = (yi - a[((k + 1i32)) as usize]);
            j = (j + 2i32);
        }
    }
    a[((m + 1i32)) as usize] = (-a[((m + 1i32)) as usize]);
}
