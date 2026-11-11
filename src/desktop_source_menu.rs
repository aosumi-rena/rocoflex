use super::*;

#[allow(dead_code)]
#[path = "runtime/canvas/toolbar_placement.rs"]
pub(super) mod literal;

#[derive(Clone, Copy)]
pub(super) struct Cached {
    owner: usize,
    offset: [f32; 2],
}

pub(super) fn rounded_display_center(center: [f32; 2], mxcsr: u32) -> Option<[i32; 2]> {
    literal::round_requested_center(center, literal::Arithmetic {
        current_thread_mxcsr: mxcsr, separate_operations: true,
    }).ok()
}

impl DesktopApp {
    fn source_root_screen(&self, owner: usize, full: Rect) -> Option<Pos2> {
        if !self.native_layout || self.layout_blocked() {
            return None;
        }
        let view = self.layout_view.as_ref()?;
        if view.positions.len() != self.preset.sources.len()
            || view.positions.iter().zip(&self.preset.sources)
                .any(|(positions, source)| positions.len() != source.points.len())
        {
            return None;
        }
        let xy = if let Some(controller) = self.layout.as_ref() {
            if controller.roots.len() != self.preset.sources.len() {
                return None;
            }
            let point = &controller.roots.get(owner)?.point;
            [point.base[0] + point.offset[0], point.base[1] + point.offset[1]]
        } else {

            if owner != 0 || self.preset.sources.len() != 1
                || self.preset.sources[0].kind != "point"
            {
                return None;
            }
            *view.positions.first()?.first()?
        };
        let offset = self.offsets.get(owner).copied().unwrap_or_default();
        let root = world_to_screen(
            Point::new(f64::from(xy[0]) + offset.x, f64::from(xy[1]) + offset.y),
            full,
            self.camera,
        );
        (root.x.is_finite() && root.y.is_finite()).then_some(root)
    }

    pub(super) fn cache_source_actions(&mut self, owner: usize, hit: Pos2, full: Rect)
        -> std::result::Result<(), source_distance::Failure> {
        if !self.native_layout || self.allow_canvas_editing {
            return Ok(());
        }
        let mut norm = source_distance::Norm::bind()?;
        let selected = (|| {
            let root = self.source_root_screen(owner, full)?;
            let candidates = literal::candidate_centers([hit.x, hit.y]).ok()?;
            let view = self.layout_view.as_ref()?;
            let mut samples = Vec::new();

            for (index, positions) in view.positions.iter().enumerate() {
                let offset = self.offsets.get(index).copied().unwrap_or_default();
                for xy in positions {
                    let sample = world_to_screen(
                        Point::new(f64::from(xy[0]) + offset.x, f64::from(xy[1]) + offset.y),
                        full,
                        self.camera,
                    );
                    if !sample.x.is_finite() || !sample.y.is_finite() {
                        return None;
                    }
                    samples.push([sample.x, sample.y]);
                }
            }
            let mut best = 0.0_f32;
            let mut offset = self.source_menu_cache.map(|previous| previous.offset);
            for candidate in candidates {
                let mut nearest = f32::from_bits(0x5015_02f9);
                for sample in &samples {
                    let distance = norm.distance(*sample, candidate).ok()?;
                    if distance < nearest {
                        nearest = distance;
                    }
                }
                if nearest > best {
                    offset = Some(literal::improving_offset(candidate, [root.x, root.y]).ok()?);
                    best = nearest;
                }
            }
            norm.check_controls().ok()?;

            Some(Cached { owner, offset: offset? })
        })();
        if norm.is_failed() { return Err(norm.into_failure()); }
        self.source_menu_cache = selected;
        Ok(())
    }

    pub(super) fn source_actions_rect(&self, full: Rect) -> Option<Rect> {
        let owner = self.selected?;
        if owner >= self.preset.sources.len() || self.layout_blocked() {
            return None;
        }
        if self.native_layout && !self.allow_canvas_editing {
            let cached = self.source_menu_cache.filter(|cached| cached.owner == owner)?;
            let observed = layout::context::provider::check_current_thread().ok()?;
            let root = self.source_root_screen(owner, full)?;
            let center = literal::current_center([root.x, root.y], cached.offset);
            let center = literal::round_requested_center(center, literal::Arithmetic {
                current_thread_mxcsr: observed,
                separate_operations: true,
            }).ok()?;

            return Some(Rect::from_min_size(
                pos2(center[0] as f32 - 32.0, center[1] as f32 - 14.0),
                vec2(64.0, 28.0),
            ));
        }

        let size = vec2(64.0, 28.0);
        let min = full.min + vec2(12.0, 76.0);
        let max = full.max - size - vec2(12.0, 12.0);
        if !self.selected_at.x.is_finite() || !self.selected_at.y.is_finite()
            || !min.x.is_finite() || !min.y.is_finite()
            || !max.x.is_finite() || !max.y.is_finite()
            || min.x > max.x || min.y > max.y
        {
            return None;
        }
        let occupied: Vec<_> = self.scene.anchors.iter()
            .map(|anchor| world_to_screen(anchor.position, full, self.camera)).collect();
        Some(source_menu_position(self.selected_at, full, &occupied, size))
    }
}
