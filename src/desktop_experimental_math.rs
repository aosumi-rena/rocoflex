//! Experimental 3D geometry. Pure math; never mutates preset coordinates.

pub type V3 = [f64; 3];
fn dot4(a: [f64; 4], b: [f64; 4]) -> f64 { (0..4).map(|i| a[i] * b[i]).sum() }
fn norm4(a: [f64; 4]) -> f64 { dot4(a, a).sqrt() }
fn remove(mut a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let k = dot4(a, b); for i in 0..4 { a[i] -= k * b[i]; } a
}
fn unit4(a: [f64; 4]) -> Option<[f64; 4]> {
    let n = norm4(a); (n.is_finite() && n > 1e-12).then(|| a.map(|x| x/n))
}
pub fn dot(a: V3, b: V3) -> f64 { (0..3).map(|i| a[i]*b[i]).sum() }
pub fn add(a: V3, b: V3) -> V3 { std::array::from_fn(|i| a[i]+b[i]) }
pub fn sub(a: V3, b: V3) -> V3 { std::array::from_fn(|i| a[i]-b[i]) }
pub fn mul(a: V3, s: f64) -> V3 { a.map(|x| x*s) }

/// Reuse an accepted 4D orientation and project it into the complement of
/// the working 2D basis. On first use, choose a dominant residual eigenspace;
/// near-ties select the first usable canonical direction in that eigenspace.
/// Retaining the anchor prevents PCA sign and tie flips during layout updates.
/// A vanished anchor is replaced only when the current data has real residual
/// variance. Zero residual variance stays flat; no dimension is fabricated.
pub fn depth(rows: &[[f32;4]], basis: [f32;8], anchor: Option<[f64;4]>)
    -> Result<(Vec<f64>, bool, Option<[f64;4]>), &'static str> {
    if anchor.is_some_and(|a| a.iter().any(|v| !v.is_finite()) || unit4(a).is_none()) {
        return Err("Invalid depth anchor");
    }
    if rows.is_empty() { return Ok((Vec::new(), false, anchor)); }
    if rows.iter().flatten().chain(basis.iter()).any(|v| !v.is_finite()) { return Err("Non-finite reduced geometry"); }
    let n = rows.len() as f64;
    let mut mean = [0.0;4];
    for row in rows { for i in 0..4 { mean[i] += row[i] as f64 / n; } }
    let centered: Vec<[f64;4]> = rows.iter().map(|p| std::array::from_fn(|i| p[i] as f64 - mean[i])).collect();
    let variance: f64 = centered.iter().map(|p| dot4(*p,*p)).sum::<f64>() / n / 4.0;
    let mut axes = Vec::new();
    for chunk in basis.chunks_exact(4) {
        let mut a = [chunk[0] as f64,chunk[1] as f64,chunk[2] as f64,chunk[3] as f64];
        for b in &axes { a = remove(a,*b); }
        if let Some(a) = unit4(a) { axes.push(a); }
    }
    let residuals: Vec<[f64;4]> = centered.iter().map(|p| {
        let mut a = *p; for b in &axes { a = remove(a,*b); } a
    }).collect();
    let mut covariance = [[0.0;4];4];
    for p in &residuals { for i in 0..4 { for j in 0..4 { covariance[i][j] += p[i]*p[j] / n; } } }
    let residual_variance: f64 = (0..4).map(|i| covariance[i][i]).sum();
    let mut vectors = [[0.0;4];4]; for (i,row) in vectors.iter_mut().enumerate() { row[i]=1.0; }
    for _ in 0..64 {
        let mut p=0; let mut q=1;
        for i in 0..4 { for j in i+1..4 { if covariance[i][j].abs()>covariance[p][q].abs() {p=i;q=j;} } }
        if covariance[p][q].abs() <= 1e-14 * residual_variance { break; }
        let theta = 0.5 * (2.0*covariance[p][q]).atan2(covariance[q][q]-covariance[p][p]);
        let (s,c) = theta.sin_cos();
        let (app,aqq,apq)=(covariance[p][p],covariance[q][q],covariance[p][q]);
        covariance[p][p]=c*c*app-2.0*s*c*apq+s*s*aqq;
        covariance[q][q]=s*s*app+2.0*s*c*apq+c*c*aqq;
        covariance[p][q]=0.0; covariance[q][p]=0.0;
        for k in 0..4 { if k!=p && k!=q {
            let (a,b)=(covariance[k][p],covariance[k][q]);
            covariance[k][p]=c*a-s*b; covariance[p][k]=covariance[k][p];
            covariance[k][q]=s*a+c*b; covariance[q][k]=covariance[k][q];
        } }
        for row in &mut vectors { let (a,b)=(row[p],row[q]); row[p]=c*a-s*b; row[q]=s*a+c*b; }
    }
    let mut largest = 0; for i in 1..4 { if covariance[i][i]>covariance[largest][largest] { largest=i; } }
    let max_variance=covariance[largest][largest];
    if max_variance <= (variance * 1e-12).max(1e-20) { return Ok((vec![0.0;rows.len()],false,anchor)); }
    let projected_anchor=anchor.and_then(|anchor| {
        let mut projected=anchor; for b in &axes {projected=remove(projected,*b);}
        let a=unit4(projected)?;
        let energy=residuals.iter().map(|p|dot4(*p,a).powi(2)).sum::<f64>()/n;
        (energy > (variance*1e-12).max(1e-20)).then_some(a)
    });
    let (axis, accepted_anchor)=if let Some(axis)=projected_anchor {(axis,anchor)} else {
        let mut selected=None;
        for canonical in 0..4 {
            let mut candidate=[0.0;4];
            for j in 0..4 {
                if covariance[j][j] >= max_variance*(1.0-1e-8) {
                    for i in 0..4 {candidate[i]+=vectors[canonical][j]*vectors[i][j];}
                }
            }
            if let Some(a)=unit4(candidate) {selected=Some(a);break;}
        }
        let mut a=selected.ok_or("Invalid residual eigenspace")?;
        if let Some(previous)=anchor {if dot4(a,previous) < -1e-10 {a=a.map(|x|-x);}}
        (a,Some(a))
    };
    let mut out: Vec<_> = residuals.iter().map(|p| dot4(*p,axis)).collect();
    spread_depth(&mut out);
    Ok((out,true,accepted_anchor))
}

fn spread_depth(values: &mut [f64]) {
    const CENTRAL: f64 = 1.10;
    const TAIL: f64 = 0.65;
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let last = sorted.len() - 1;
    let percentile = |fraction: f64| {
        let i = last as f64 * fraction;
        let lower = sorted[i.floor() as usize];
        let upper = sorted[i.ceil() as usize];
        lower + (upper - lower) * i.fract()
    };
    let (mut low, mut high) = (percentile(0.10), percentile(0.90));
    let full_span = sorted[last] - sorted[0];
    if high - low <= full_span * 1e-6 { (low, high) = (sorted[0], sorted[last]); }
    let span = high - low;
    if span == 0.0 { return; }
    // Keep the central depth linear. Only the tails bend to fit the canvas.
    let mid = low + span * 0.5;
    for value in values {
        let z = (*value - mid) / span * (2.0 * CENTRAL);
        let tail = z.abs() - CENTRAL;
        *value = if tail <= 0.0 { z } else {
            z.signum() * (CENTRAL + TAIL * (tail / (tail + TAIL)))
        };
    }
}

#[derive(Default)]
pub struct ProbeTint { channels: V3, total: f64 }
impl ProbeTint {
    pub fn add(&mut self, rgb: [u8; 3], weight: f64) {
        if weight.is_finite() && weight > 0.0 {
            for (channel, value) in self.channels.iter_mut().zip(rgb) {
                *channel += f64::from(value) * weight;
            }
            self.total += weight;
        }
    }
    pub fn color(&self) -> [u8; 3] {
        if self.total == 0.0 { [255; 3] }
        else { self.channels.map(|channel| (channel / self.total).round() as u8) }
    }
}

/// Gate out keys which were already held when navigation gained ownership.
/// A fresh non-repeat down in the acquisition frame remains usable.
#[derive(Default)]
pub struct KeyGate { active: bool, blocked: u16 }
impl KeyGate {
    pub fn reset(&mut self) {self.active=false;self.blocked=0;}
    pub fn permit(&mut self, down:u16, fresh:u16, disqualified:u16)->u16 {
        if !self.active {self.blocked=down & !fresh;self.active=true;}
        self.blocked |= disqualified & down;
        self.blocked &= down;
        !(self.blocked | disqualified)
    }
}

/// Resolve the initiating event, not modifiers observed later in its batch.
#[derive(Clone,Copy,Debug)]
pub struct SignEvent {pub bit:u16,pub increase:bool,pub ctrl:bool,pub shift:bool,pub alt:bool,pub mac_cmd:bool}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SignAction {Zoom(bool),Speed(bool)}
pub fn sign_action(events:&[SignEvent])->Option<(u16,SignAction)> {
    let [event]=events else {return None;};
    if event.alt || event.mac_cmd {return None;}
    if event.ctrl {Some((event.bit,SignAction::Zoom(event.increase)))}
    else if event.shift {Some((event.bit,SignAction::Speed(event.increase)))} else {None}
}

#[derive(Clone, Copy, Debug)]
pub struct Camera { pub position: V3, pub yaw: f64, pub pitch: f64, pub zoom: f64 }
impl Default for Camera {
    fn default() -> Self {
        let mut c = Self {position:[0.0;3],yaw:0.35,pitch:-0.22,zoom:1.0};
        c.position=mul(c.forward(),-2.8); c
    }
}
impl Camera {
    const NEAR: f64 = 0.05;

    pub fn forward(self) -> V3 { let (sy,cy)=self.yaw.sin_cos(); let (sp,cp)=self.pitch.sin_cos(); [sy*cp,sp,cy*cp] }
    pub fn right(self) -> V3 { let (s,c)=self.yaw.sin_cos(); [c,0.0,-s] }
    pub fn up(self) -> V3 { let (sy,cy)=self.yaw.sin_cos(); let (sp,cp)=self.pitch.sin_cos(); [-sy*sp,cp,-cy*sp] }
    pub fn project(self, point: V3, size: [f64;2]) -> Option<V3> {
        let d=sub(point,self.position);
        let z=dot(d,self.forward());
        if z<=Self::NEAR || !z.is_finite() {return None;}
        self.project_at_depth(d,z,size)
    }
    fn focal_length(self, size: [f64; 2]) -> Option<f64> {
        if !self.zoom.is_finite() || self.zoom<=0.0 || size.iter().any(|v| !v.is_finite() || *v<=0.0) {return None;}
        let focal=size[0].min(size[1])*0.5*2.8*self.zoom;
        focal.is_finite().then_some(focal)
    }
    pub fn pixels_per_unit(self, depth: f64, size: [f64; 2]) -> Option<f64> {
        if !depth.is_finite() || depth < Self::NEAR { return None; }
        let scale = self.focal_length(size)? / depth;
        scale.is_finite().then_some(scale)
    }
    fn project_at_depth(self, d: V3, z: f64, size: [f64;2]) -> Option<V3> {
        let focal=self.focal_length(size)?;
        let result=[size[0]*0.5+dot(d,self.right())*focal/z, size[1]*0.5-dot(d,self.up())*focal/z,z];
        result.iter().all(|v|v.is_finite()).then_some(result)
    }
    pub fn project_segment(self, a: V3, b: V3, size: [f64;2]) -> Option<[V3;2]> {
        let mut points=[sub(a,self.position),sub(b,self.position)];
        let forward=self.forward();
        let mut depths=points.map(|p|dot(p,forward));
        if depths.iter().any(|z|!z.is_finite()) || depths.iter().all(|z|*z<=Self::NEAR) {return None;}
        if let Some(i)=depths.iter().position(|z|*z<Self::NEAR) {
            let j=1-i;
            let t=(Self::NEAR-depths[i])/(depths[j]-depths[i]);
            points[i]=add(points[i],mul(sub(points[j],points[i]),t));
            depths[i]=Self::NEAR;
        }
        Some([self.project_at_depth(points[0],depths[0],size)?,
            self.project_at_depth(points[1],depths[1],size)?])
    }
    pub fn axis_projection(self, origin: V3, axis: usize, size: [f64;2]) -> Option<[V3;3]> {
        if axis>2 {return None;}
        let a=self.project(origin,size)?;
        let mut sample=origin; sample[axis]+=0.1;
        let b=self.project(sample,size)?;
        if (b[0]-a[0]).powi(2)+(b[1]-a[1]).powi(2)<4.0 {return None;}
        let mut tip=origin; tip[axis]+=0.25;
        let c=self.project(tip,size)?;
        if (c[0]-a[0]).powi(2)+(c[1]-a[1]).powi(2)<4.0 {return None;}
        Some([a,b,c])
    }
    pub fn rotate(&mut self, delta: [f64;2]) {
        if delta.iter().any(|v|!v.is_finite()) || !self.yaw.is_finite() || !self.pitch.is_finite() {return;}
        let yaw=(self.yaw+delta[0]*0.006).rem_euclid(std::f64::consts::TAU);
        let pitch=(self.pitch-delta[1]*0.006).clamp(-1.48,1.48);
        if yaw.is_finite() && pitch.is_finite() {self.yaw=yaw;self.pitch=pitch;}
    }
    pub fn walk(&mut self, forward: f64, right: f64, dt: f64, speed: f64) {
        self.walk_vertical(forward, right, 0.0, dt, speed);
    }
    pub fn walk_vertical(&mut self, forward: f64, right: f64, vertical: f64, dt: f64, speed: f64) {
        if [forward,right,vertical,dt,speed].iter().any(|v| !v.is_finite()) || dt<=0.0 || speed<=0.0 {return;}
        let direction=add(add(mul(self.forward(),forward),mul(self.right(),right)),[0.0,vertical,0.0]);
        let length=dot(direction,direction).sqrt();
        if length>0.0 && length.is_finite() {
            let next=add(self.position,mul(direction,speed*dt.min(0.1)/length));
            if next.iter().all(|v|v.is_finite() && v.abs()<=1e6) {self.position=next;}
        }
    }
}

/// Screen-space line-axis drag: preserve the press position and initial probe.
/// Reject axes nearly facing the camera rather than producing a huge jump.
pub fn drag_axis(initial: V3, axis: usize, press: [f64;2], cursor: [f64;2], camera: Camera, size: [f64;2]) -> Option<V3> {
    let [a,b,_]=camera.axis_projection(initial,axis,size)?;
    let d=[b[0]-a[0],b[1]-a[1]]; let norm=d[0]*d[0]+d[1]*d[1];
    let lambda=((cursor[0]-press[0])*d[0]+(cursor[1]-press[1])*d[1])/norm;
    // Invert perspective along the axis, rather than assuming pixels/world
    // stay linear as depth changes. Reject the pole and the near plane.
    let dz=camera.forward()[axis]; let denominator=a[2]+0.1*dz-lambda*0.1*dz;
    if denominator.abs()<1e-10 {return None;}
    let t=lambda*0.1*a[2]/denominator;
    if !t.is_finite() || a[2]+t*dz<=0.05 {return None;}
    let mut out=initial; out[axis]=(out[axis]+t).clamp(-100.0,100.0); Some(out)
}

#[derive(Clone, Debug)]
pub struct MixSource { pub trajectory: bool, pub enabled: bool, pub points: Vec<(V3,bool)> }
/// Source strength depends on the nearest point on its geometry, not how
/// many segments describe it. Within each trajectory, nearby segment
/// projections receive smooth weights to avoid jumps at self-intersections.
/// Projection and weights remain f64 until the controller ABI boundary.
pub fn weights(sources: &[MixSource], probe: V3, radius: f64) -> Result<Vec<Vec<f64>>, &'static str> {
    if probe.iter().any(|x| !x.is_finite()) || !radius.is_finite() || radius<=0.0 {return Err("Invalid 3D probe");}
    // (point a, point b, interpolation, squared distance)
    let mut owners:Vec<(usize,Vec<(usize,usize,f64,f64)>,f64)>=Vec::new();
    for (owner,source) in sources.iter().enumerate() {
        if !source.enabled {continue;} let mut candidates=Vec::new();
        let mut joined=vec![false;source.points.len()];
        for (at,_) in &source.points {if at.iter().any(|x|!x.is_finite()){return Err("Invalid 3D source");}}
        if source.trajectory {for i in 0..source.points.len().saturating_sub(1) {
            let (a,ea)=source.points[i]; let (b,eb)=source.points[i+1]; if !ea || !eb {continue;}
            let edge=sub(b,a); let l2=dot(edge,edge); if l2<=1e-24 {continue;}
            let t=(dot(sub(probe,a),edge)/l2).clamp(0.0,1.0); let d=sub(add(a,mul(edge,t)),probe);
            candidates.push((i,i+1,t,dot(d,d)));joined[i]=true;joined[i+1]=true;
        }}
        for (i,(at,enabled)) in source.points.iter().enumerate() {
            if !enabled || joined[i] {continue;} let d=sub(*at,probe);
            candidates.push((i,i,0.0,dot(d,d)));
        }
        if !candidates.is_empty() {
            let nearest=candidates.iter().map(|v|v.3).fold(f64::INFINITY,f64::min);
            if !nearest.is_finite(){return Err("Invalid 3D distance");}
            owners.push((owner,candidates,nearest));
        }
    }
    if owners.is_empty(){return Err("No enabled 3D voice targets");}
    let soft=(radius*0.25).max(0.005); let width=soft*soft;
    let logs:Vec<_>=owners.iter().map(|v|-2.0*(v.2+width).ln()).collect();
    let peak=logs.iter().copied().fold(f64::NEG_INFINITY,f64::max);
    let raw:Vec<_>=logs.iter().map(|x|(x-peak).exp()).collect();
    let total: f64=raw.iter().sum(); if !total.is_finite() || total<=0.0{return Err("Invalid 3D weights");}
    let mut output:Vec<Vec<f64>>=sources.iter().map(|s|vec![0.0;s.points.len()]).collect();
    for ((owner,candidates,nearest),strength) in owners.into_iter().zip(raw) {
        let factors:Vec<_>=candidates.iter().map(|v|(-(v.3-nearest)/width).exp()).collect();
        let sum:f64=factors.iter().sum();
        if !sum.is_finite() || sum<=0.0 {return Err("Invalid trajectory interpolation");}
        for ((a,b,t,_),factor) in candidates.into_iter().zip(factors) {
            let w=(strength/total)*(factor/sum); output[owner][a]+=w*(1.0-t); output[owner][b]+=w*t;
        }
    }
    Ok(output)
}
