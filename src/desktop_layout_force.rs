#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub base: [f32; 2],
    pub offset: [f32; 2],
    pub force: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct Root {
    pub point: Point,
    pub covariance: [f32; 4],
    pub children: Vec<Point>,
    pub trajectory: bool,
}

fn location(roots: &[Root], root: usize, child: Option<usize>) -> [f32; 2] {
    let parent = &roots[root];
    let point = child.map_or(&parent.point, |i| &parent.children[i]);
    let mut position = [
        point.base[0] + point.offset[0],
        point.base[1] + point.offset[1],
    ];
    if child.is_some() {
        position[0] += parent.point.offset[0];
        position[1] += parent.point.offset[1];
    }
    position
}

fn point_mut(roots: &mut [Root], root: usize, child: Option<usize>) -> &mut Point {
    match child {
        Some(i) => &mut roots[root].children[i],
        None => &mut roots[root].point,
    }
}

pub fn step(roots: &mut [Root]) -> f64 {
    for root in &mut *roots {
        root.point.force = [0.0; 2];
        for child in &mut root.children {
            child.force = [0.0; 2];
        }
    }
    for root in &mut *roots {
        if !root.trajectory {
            continue;
        }
        for i in 1..root.children.len().saturating_sub(1) {
            let [previous, middle, next] =
                [root.children[i - 1], root.children[i], root.children[i + 1]];
            for axis in 0..2 {
                let curvature_change = ((next.base[axis] - middle.base[axis])
                    - ((next.offset[axis] + next.base[axis])
                        - (middle.offset[axis] + middle.base[axis])))
                    + (previous.base[axis] - middle.base[axis])
                    - ((previous.offset[axis] + previous.base[axis])
                        - (middle.offset[axis] + middle.base[axis]));
                root.children[i].force[axis] -= curvature_change * 0.3;
            }
        }
    }
    for i in 0..roots.len() {
        if !roots[i].trajectory {
            continue;
        }
        for j in i + 1..roots.len() {
            if !roots[j].trajectory {
                continue;
            }
            let delta = std::array::from_fn::<_, 2, _>(|axis| {
                ((roots[i].point.base[axis] + roots[i].point.offset[axis])
                    - roots[j].point.base[axis])
                    - roots[j].point.offset[axis]
            });
            let ca = roots[i].covariance;
            let cb = roots[j].covariance;
            let sum = [ca[0] + cb[0], ca[1] + cb[1], ca[2] + cb[2], ca[3] + cb[3]];
            let reciprocal = 1.0_f32 / (sum[0] * sum[3] - sum[2] * sum[1]);
            let inverse = [
                sum[3] * reciprocal,
                -sum[1] * reciprocal,
                -sum[2] * reciprocal,
                sum[0] * reciprocal,
            ];
            let negative_product = [
                -inverse[0] * delta[0] + -inverse[2] * delta[1],
                -inverse[1] * delta[0] + -inverse[3] * delta[1],
            ];
            let quadratic = negative_product[0] * delta[0] + negative_product[1] * delta[1];
            let strength = (f64::from(quadratic) * 0.5).exp() as f32;
            for axis in 0..2 {
                roots[i].point.force[axis] = delta[axis] * strength + roots[i].point.force[axis];
                roots[j].point.force[axis] -= delta[axis] * strength;
            }
        }
    }
    let mut flat = Vec::new();
    for (i, root) in roots.iter().enumerate() {
        if root.trajectory {
            flat.extend((0..root.children.len()).map(|j| (i, Some(j))));
        } else {
            flat.push((i, None));
        }
    }
    for (i, &(root_a, child_a)) in flat.iter().enumerate() {
        let a = location(roots, root_a, child_a);
        for &(root_b, child_b) in &flat[i + 1..] {
            let b = location(roots, root_b, child_b);
            let delta = [a[0] - b[0], a[1] - b[1]];
            let strength = 0.001_f32 / ((delta[0] * delta[0] + delta[1] * delta[1]) + 1.0e-5);
            let force = [delta[0] * strength, strength * delta[1]];
            let coupling = if child_a.is_some() && child_b.is_some() && root_a == root_b {
                0.3
            } else {
                1.0
            };
            for axis in 0..2 {
                point_mut(roots, root_a, child_a).force[axis] += coupling * force[axis];
                point_mut(roots, root_b, child_b).force[axis] -= coupling * force[axis];
            }
            if child_a.is_some() {
                for axis in 0..2 {
                    roots[root_a].point.force[axis] =
                        force[axis] * 0.1 + roots[root_a].point.force[axis];
                }
            }
            if child_b.is_some() {
                for axis in 0..2 {
                    roots[root_b].point.force[axis] -= force[axis] * 0.1;
                }
            }
        }
    }
    for &(root, child) in &flat {
        let point = point_mut(roots, root, child);
        for axis in 0..2 {
            point.offset[axis] = (point.force[axis] + point.offset[axis]) * 0.8;
        }
    }
    let mut movement = 0.0_f64;
    for root in roots {
        let old = root.point.offset;
        if root.trajectory {
            let mut mean = [0.0_f32; 2];
            for child in &root.children {
                for axis in 0..2 {
                    mean[axis] += child.offset[axis];
                }
            }
            for axis in 0..2 {
                mean[axis] /= root.children.len() as f32;
                root.point.offset[axis] += mean[axis];
            }
            for child in &mut root.children {
                for axis in 0..2 {
                    child.offset[axis] -= mean[axis];
                }
            }
        }
        for axis in 0..2 {
            root.point.offset[axis] = (root.point.force[axis] + root.point.offset[axis]) * 0.8;
        }
        let change = [root.point.offset[0] - old[0], root.point.offset[1] - old[1]];
        movement += f64::from((change[0] * change[0] + change[1] * change[1]).sqrt());
    }
    movement
}

