use super::*;

#[allow(dead_code)]
#[path = "runtime/canvas/caption_placement.rs"]
mod literal;

enum State {
    Trajectory(literal::Poll),
    Point(std::result::Result<literal::DrawBoundary, literal::StandaloneStopped>),
}

pub(super) struct Cached {
    points: Vec<[u32; 2]>,
    width: u32,
    height: i32,
    trajectory: bool,
    state: Option<State>,
}

impl Cached {
    fn draw(
        slot: &mut Option<Self>,
        points: Vec<[f32; 2]>,
        measured_width: f32,
        height: i32,
        trajectory: bool,
        work: &mut usize,
        ctx: &egui::Context,
    ) -> Option<literal::DrawBoundary> {

        let observed = layout::context::provider::check_current_thread().ok()?;
        let same = slot.as_ref().is_some_and(|cache| {
            cache.width == measured_width.to_bits() && cache.height == height
                && cache.trajectory == trajectory && cache.points.len() == points.len()
                && cache.points.iter().zip(&points).all(|(saved, point)| {
                    *saved == [point[0].to_bits(), point[1].to_bits()]
                })
        });
        if !same {
            let key = points.iter().map(|p| [p[0].to_bits(), p[1].to_bits()]).collect();
            let arithmetic = literal::Arithmetic { mxcsr: observed, separate_operations: true };
            let state = if trajectory {
                State::Trajectory(literal::Poll::Pending(literal::Prepared::display_trajectory(
                    points, literal::DisplayMetrics { measured_width, canvas_height: height }, arithmetic,
                )))
            } else {
                State::Point(literal::projected_standalone(*points.first()?, arithmetic))
            };
            *slot = Some(Self {
                points: key, width: measured_width.to_bits(), height, trajectory,
                state: Some(state),
            });
        }
        let cache = slot.as_mut()?;
        if matches!(cache.state.as_ref(), Some(State::Trajectory(literal::Poll::Pending(_)))) {

            let grant = (*work).min(65_536);
            if grant != 0 {
                let Some(State::Trajectory(literal::Poll::Pending(pending))) = cache.state.take()
                    else { return None; };
                let (next, spent) = pending.poll_with_work(grant);
                *work -= spent;
                cache.state = Some(State::Trajectory(next));
            }
        }
        match cache.state.as_ref()? {
            State::Trajectory(literal::Poll::Ready(done)) => Some(done.draw),
            State::Point(Ok(draw)) => Some(*draw),
            State::Trajectory(literal::Poll::Pending(_)) => {
                ctx.request_repaint_after(Duration::from_millis(16));
                None
            }

            State::Trajectory(literal::Poll::Failed(_)) | State::Point(Err(_)) => None,
        }
    }
}

impl DesktopApp {
    pub(super) fn paint_source_labels(&mut self, painter: &egui::Painter, rect: Rect) {
        if !self.native_layout {
            self.caption_placements.clear();
            self.paint_inspection_source_labels(painter, rect);
            return;
        }
        self.label_rects.clear();
        self.caption_placements.resize_with(self.preset.sources.len(), || None);
        if self.layout_blocked()
            || layout::context::provider::check_current_thread().is_err()
            || !rect.min.x.is_finite() || !rect.min.y.is_finite()
            || !rect.width().is_finite() || rect.width() <= 0.0
            || !rect.height().is_finite() || rect.height() <= 0.0
            || rect.height() >= 2_147_483_648.0
        {
            return;
        }
        let Some(view) = self.layout_view.as_ref().filter(|view| {
            view.positions.len() == self.preset.sources.len()
                && view.positions.iter().zip(&self.preset.sources)
                    .all(|(positions, source)| positions.len() == source.points.len())
        }) else { return; };

        let height = rect.height() as i32;
        let font = FontId::proportional(13.5);
        let color = Color32::from_rgb(183, 184, 187);
        let mut work = 262_144;
        for (index, source) in self.preset.sources.iter().enumerate() {

            let text = source.name.split('\0').next().unwrap_or_default();
            if text.is_empty() {
                self.caption_placements[index] = None;
                continue;
            }
            let offset = self.offsets.get(index).copied().unwrap_or_default();
            let points: Vec<_> = view.positions[index].iter().map(|xy| {
                let at = world_to_screen(
                    Point::new(f64::from(xy[0]) + offset.x, f64::from(xy[1]) + offset.y),
                    rect, self.camera,
                );
                [at.x - rect.left(), at.y - rect.top()]
            }).collect();
            if points.is_empty() || points.iter().flatten().any(|p| !p.is_finite()) {
                self.caption_placements[index] = None;
                continue;
            }

            let label = text.to_owned();
            let measured_width = painter.layout_no_wrap(label.clone(), font.clone(), color).size().x;
            let Some(draw) = Cached::draw(
                &mut self.caption_placements[index], points, measured_width,
                height, source.kind == "trajectory", &mut work, painter.ctx(),
            ) else { continue; };
            let mut job = egui::text::LayoutJob::simple(
                label, font.clone(), color, draw.maximum_line_width as f32,
            );
            job.halign = egui::Align::Center;
            let galley = painter.layout_job(job);

            let baseline = galley.rows.first().and_then(|row| {
                row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y)
            }).or_else(|| {
                let sample = painter.layout_no_wrap("M".into(), font.clone(), color);
                sample.rows.first().and_then(|row| {
                    row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y)
                })
            });
            let Some(baseline) = baseline else { continue; };
            let delta = self.label_offsets.get(index).copied().unwrap_or_default();
            let origin = pos2(
                rect.left() + draw.x as f32 + 50.0 + delta.x as f32,
                rect.top() + draw.baseline_y as f32 - baseline + delta.y as f32,
            );
            let bounds = galley.rect.translate(origin.to_vec2());
            if !bounds.min.x.is_finite() || !bounds.min.y.is_finite()
                || !bounds.max.x.is_finite() || !bounds.max.y.is_finite()
            {
                continue;
            }

            painter.galley(origin, galley, color);
            self.label_rects.push((index, bounds));
        }
    }
}
