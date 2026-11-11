use super::{layout, partial};
use layout::context::{self, CacheBinding};

const UPDATE_ERROR: &str = "This canvas configuration cannot be updated yet.";

#[derive(Clone)]
pub(super) enum CanvasState {
    Inspection,
    Partial {
        controller: partial::PartialController,
        cache: CacheBinding,
    },
    Full {
        controller: context::ContextController,
        binding: partial::DispatchBinding,
    },
}

pub(super) struct PreparedCanvas {
    stage: Stage,
    changed: bool,
}

enum Stage {
    Full(context::PreparedContextController, partial::DispatchBinding),
    Display(
        context::display_gate::PreparedDisplayContext,
        partial::DispatchBinding,
    ),
    Partial(partial::PreparedPartial, CacheBinding),
    Promotion(context::partial_gate::PreparedPartialContext),
    Ready(CanvasState),
}

impl CanvasState {
    pub fn actual_no_file_startup(controls: partial::ProbeControls) -> Result<Self, String> {
        partial::PartialController::startup(partial::StartupOrigin::ActualNoFileStartup, controls)
            .map(|controller| Self::Partial {
                controller,
                cache: CacheBinding::Unbound,
            })
            .map_err(|_| UPDATE_ERROR.into())
    }

    pub fn as_ref(&self) -> Option<&layout::Controller> {
        match self {
            Self::Full { controller, .. } => Some(controller.controller()),
            _ => None,
        }
    }

    pub fn completed_probe_geometry(&self)
        -> Option<&std::sync::Arc<layout::background_geometry::CompletedGeometry>>
    {
        match self {
            Self::Full { controller, .. } => controller.controller().completed_probe_geometry(),
            Self::Partial { controller, .. } => controller.completed_probe_geometry(),
            Self::Inspection => None,
        }
    }

    pub fn active(&self) -> bool {
        match self {
            Self::Inspection => false,
            Self::Partial { controller, .. } => controller.active(),
            Self::Full { controller, .. } => controller.active(),
        }
    }

    pub fn sources(&self) -> Option<&[layout::Source]> {
        match self {
            Self::Inspection => None,
            Self::Partial { controller, .. } => Some(controller.sources()),
            Self::Full { controller, .. } => Some(controller.sources()),
        }
    }

    pub fn checked_view(&self) -> Result<partial::View, String> {
        match self {
            Self::Inspection => Err(UPDATE_ERROR.into()),
            Self::Partial { controller, .. } => {
                controller.checked_view().map_err(|_| UPDATE_ERROR.into())
            }
            Self::Full { controller, .. } => {
                let view = controller.checked_view()?;
                Ok(partial::View {
                    positions: view.positions,
                    scalars: view.scalars.to_vec(),
                })
            }
        }
    }

    #[allow(dead_code)]
    pub fn working_uv_snapshot(&self) -> Option<[f32; 8]> {
        match self {
            Self::Inspection => None,
            Self::Partial { controller, .. } => Some(controller.working_uv_snapshot()),
            Self::Full { controller, .. } => Some(controller.working_uv_snapshot()),
        }
    }

    pub fn cache_binding(&self) -> CacheBinding {
        match self {
            Self::Inspection => CacheBinding::Unbound,
            Self::Partial { cache, .. } => cache.clone(),
            Self::Full { controller, .. } => controller.cache_binding().clone(),
        }
    }

    pub fn matching_metadata(&self, sources: &[layout::Source]) -> Result<Self, String> {
        match self {
            Self::Partial { controller, cache } => controller
                .fork_with_matching_metadata(sources)
                .map(|controller| Self::Partial {
                    controller,
                    cache: cache.clone(),
                })
                .map_err(|_| UPDATE_ERROR.into()),
            Self::Full {
                controller,
                binding,
            } => controller
                .fork_with_matching_metadata(sources)
                .map(|controller| Self::Full {
                    controller,
                    binding: *binding,
                })
                .map_err(|_| UPDATE_ERROR.into()),
            _ => Err(UPDATE_ERROR.into()),
        }
    }

    pub fn prepare_append(&self, source: layout::Source) -> Result<PreparedCanvas, String> {
        match self {
            Self::Partial { controller, cache } if controller.sources().is_empty() => {
                let stage = match &source {
                    layout::Source::Point { .. } => Stage::Partial(
                        controller
                            .prepare_point(source)
                            .map_err(|_| UPDATE_ERROR.to_owned())?,
                        cache.clone(),
                    ),
                    layout::Source::Trajectory { .. } => Stage::Promotion(
                        context::partial_gate::PreparedPartialContext::prepare_contextual(
                            controller.clone(),
                            source,
                            context::partial_gate::Route::FirstTrajectory,
                            cache.clone(),
                        ),
                    ),
                };
                Ok(PreparedCanvas {
                    stage,
                    changed: true,
                })
            }
            Self::Partial { controller, cache } => {
                let stage = context::partial_gate::PreparedPartialContext::prepare_contextual(
                    controller.clone(),
                    source,
                    context::partial_gate::Route::Promotion(
                        partial::GeometryHook::ExactTwoStandaloneFirstProductInstalled,
                    ),
                    cache.clone(),
                );
                Ok(PreparedCanvas {
                    stage: Stage::Promotion(stage),
                    changed: true,
                })
            }
            Self::Full {
                controller,
                binding,
            } => {
                let stage = controller
                    .prepare_append(source)
                    .map_err(|_| UPDATE_ERROR.to_owned())?;
                Ok(PreparedCanvas::full(stage, *binding))
            }
            Self::Inspection => Err(UPDATE_ERROR.into()),
        }
    }

    pub fn prepare_callback(&self) -> Option<PreparedCanvas> {
        match self {
            Self::Inspection => None,
            Self::Partial { controller, cache } => Some(PreparedCanvas {
                stage: Stage::Partial(controller.prepare_callback(), cache.clone()),
                changed: false,
            }),
            Self::Full {
                controller,
                binding,
            } => Some(PreparedCanvas::full(
                controller.prepare_callback(),
                *binding,
            )),
        }
    }

    pub fn prepare_remove(&self, index: usize) -> Result<PreparedCanvas, String> {
        match self {
            Self::Full {
                controller,
                binding,
            } => controller
                .prepare_remove(index)
                .map(|prepared| PreparedCanvas::full(prepared, *binding))
                .map_err(|_| UPDATE_ERROR.to_owned()),
            _ => Err(UPDATE_ERROR.into()),
        }
    }

    pub fn probe_changed(&mut self, controls: partial::ProbeControls) -> Result<(), String> {
        match self {
            Self::Inspection => Ok(()),
            Self::Partial { controller, .. } => {
                let prepared = controller
                    .prepare_probe_changed(controls)
                    .map_err(|_| UPDATE_ERROR.to_owned())?;
                let next = prepared.into_ready().map_err(|_| UPDATE_ERROR.to_owned())?;
                *controller = next;
                Ok(())
            }
            Self::Full { controller, .. } => {
                let next = controller
                    .probe_changed(controls.xy, controls.radius)
                    .map_err(|_| UPDATE_ERROR.to_owned())?;
                *controller = next;
                Ok(())
            }
        }
    }
}

pub(super) fn current_binding() -> partial::DispatchBinding {
    partial::DispatchBinding {
        checked_literal_required: true,
        source_reload: 10,
        exact_two_standalone_first_product_required: false,
    }
}

impl PreparedCanvas {
    pub fn full(
        prepared: context::PreparedContextController,
        binding: partial::DispatchBinding,
    ) -> Self {
        let changed = prepared.geometry_changed();
        Self {
            stage: Stage::Full(prepared, binding),
            changed,
        }
    }

    pub fn inspection_display(
        sources: Vec<layout::Source>,
        incoming: [f32; 8],
        probe: [f32; 2],
        radius: f64,
        cache: CacheBinding,
    ) -> Self {
        Self::document_display(sources, incoming, probe, radius, cache,
            context::display_gate::Origin::ExistingInspectionDisplay)
    }

    pub fn document_display(
        sources: Vec<layout::Source>, incoming: [f32; 8], probe: [f32; 2],
        radius: f64, cache: CacheBinding, origin: context::display_gate::Origin,
    ) -> Self {
        let binding = current_binding();
        let prepared = context::display_gate::PreparedDisplayContext::prepare(
            context::display_gate::Input {
                origin,
                sources,
                incoming,
                probe,
                radius,
                reload: binding.source_reload,
                cache,
            },
        );
        Self {
            stage: Stage::Display(prepared, binding),
            changed: true,
        }
    }

    pub fn geometry_changed(&self) -> bool {
        self.changed
    }

    #[allow(dead_code)]
    pub fn phase(&self) -> context::Phase {
        match &self.stage {
            Stage::Full(prepared, _) => prepared.phase(),
            Stage::Display(prepared, _) => prepared.phase(),
            Stage::Partial(prepared, _) => map_phase(prepared.phase()),
            Stage::Promotion(prepared) => prepared.phase(),
            Stage::Ready(_) => context::Phase::Ready,
        }
    }

    pub fn finish_probe(self) -> Result<Self, (Self, String)> {
        let Self { stage, changed } = self;
        let fail = |stage| (Self { stage, changed }, UPDATE_ERROR.to_owned());
        let ready = match stage {
            Stage::Ready(state) => state,
            Stage::Full(mut prepared, binding) => {
                if !binding.checked_literal_required
                    || binding.source_reload != 10
                    || prepared.candidate().reload() != binding.source_reload
                {
                    return Err(fail(Stage::Full(prepared, binding)));
                }
                if prepared.phase() == context::Phase::AwaitingContext
                    && prepared.resolve_context_from_process(true).is_err()
                {
                    return Err(fail(Stage::Full(prepared, binding)));
                }
                if prepared.finish_probe().is_err() {
                    return Err(fail(Stage::Full(prepared, binding)));
                }
                let controller = match prepared.into_ready() {
                    Ok(controller) => controller,
                    Err(prepared) => return Err(fail(Stage::Full(prepared, binding))),
                };
                CanvasState::Full {
                    controller,
                    binding,
                }
            }
            Stage::Display(mut prepared, binding) => {
                if !binding.checked_literal_required || binding.source_reload != 10 {
                    return Err(fail(Stage::Display(prepared, binding)));
                }
                if prepared.phase() == context::Phase::AwaitingContext
                    && prepared.resolve_context_from_process(true).is_err()
                {
                    return Err(fail(Stage::Display(prepared, binding)));
                }
                if prepared.finish_probe().is_err() {
                    return Err(fail(Stage::Display(prepared, binding)));
                }
                let controller = match prepared.into_ready() {
                    Ok(controller) => controller,
                    Err(prepared) => return Err(fail(Stage::Display(prepared, binding))),
                };
                CanvasState::Full {
                    controller,
                    binding,
                }
            }
            Stage::Partial(prepared, cache) => {
                let prepared = if prepared.phase() == partial::Phase::Ready {
                    prepared
                } else {
                    if prepared.phase() != partial::Phase::AwaitingProbe {
                        return Err(fail(Stage::Partial(prepared, cache)));
                    }
                    let Some(request) = prepared.request() else {
                        return Err(fail(Stage::Partial(prepared, cache)));
                    };
                    match prepared.complete_probe(checked_probe_completion(&request)) {
                        Ok(prepared) => prepared,
                        Err(error) => return Err(fail(Stage::Partial(error.pending, cache))),
                    }
                };
                let controller = match prepared.into_ready() {
                    Ok(controller) => controller,
                    Err(prepared) => return Err(fail(Stage::Partial(prepared, cache))),
                };
                CanvasState::Partial { controller, cache }
            }
            Stage::Promotion(mut prepared) => {
                if prepared.phase() == context::Phase::AwaitingContext
                    && prepared.resolve_context_from_process(true).is_err()
                {
                    return Err(fail(Stage::Promotion(prepared)));
                }
                if prepared.phase() != context::Phase::AwaitingProbe {
                    return Err(fail(Stage::Promotion(prepared)));
                }
                let Some(request) = prepared.probe_request() else {
                    return Err(fail(Stage::Promotion(prepared)));
                };
                match prepared.complete_probe(checked_probe_completion(&request)) {
                    Ok(ready) => CanvasState::Full {
                        controller: ready.controller,
                        binding: ready.binding,
                    },
                    Err(error) => return Err(fail(Stage::Promotion(error.pending))),
                }
            }
        };
        Ok(Self {
            stage: Stage::Ready(ready),
            changed,
        })
    }

    pub fn ready(&self) -> Option<&CanvasState> {
        match &self.stage {
            Stage::Ready(state) => Some(state),
            _ => None,
        }
    }

    pub fn into_ready(self) -> Result<CanvasState, Self> {
        match self.stage {
            Stage::Ready(state) => Ok(state),
            _ => Err(self),
        }
    }
}

fn map_phase(phase: partial::Phase) -> context::Phase {
    match phase {
        partial::Phase::Ready => context::Phase::Ready,
        partial::Phase::AwaitingProbe => context::Phase::AwaitingProbe,
        partial::Phase::GeometryFailed => context::Phase::GeometryFailed,
        partial::Phase::ProbeFailed => context::Phase::ProbeFailed,
    }
}

fn checked_empty_scalar_guard(request: &partial::ProbeRequest) -> Result<(), String> {
    if request.kind != partial::RequestKind::EmptyGuard
        || !request.roots.is_empty()
        || request.controls.xy.iter().any(|v| !v.is_finite())
        || !request.controls.radius.is_finite()
        || request.controls.radius <= 0.0
    {
        return Err(UPDATE_ERROR.into());
    }

    Ok(())
}

fn checked_probe_completion(request: &partial::ProbeRequest) -> partial::CheckedProbeCompletion {
    if request.kind == partial::RequestKind::EmptyGuard {
        return partial::CheckedProbeCompletion::EmptyGuard(checked_empty_scalar_guard(request));
    }
    if context::provider::check_current_thread().is_err() {
        return partial::CheckedProbeCompletion::Literal(Err(UPDATE_ERROR.into()));
    }
    let mut roots = Vec::with_capacity(request.roots.len());
    for root in &request.roots {
        let samples: Vec<_> = root
            .points
            .iter()
            .map(|point| layout::probe_math::Sample {
                xy: point.xy,
                old: point.old,
            })
            .collect();
        if root.trajectory {
            roots.push(layout::probe_math::Root::Trajectory {
                owner: root.owner,
                children: samples,
            });
        } else if samples.len() == 1 {
            roots.push(layout::probe_math::Root::Standalone {
                owner: root.owner,
                point: samples[0].clone(),
            });
        } else {
            return partial::CheckedProbeCompletion::Literal(Err(UPDATE_ERROR.into()));
        }
    }
    let result = layout::probe_math::refresh_checked(
        &mut roots,
        request.controls.xy,
        request.controls.radius,
    )
    .map_err(|_| UPDATE_ERROR.into());
    partial::CheckedProbeCompletion::Literal(result)
}
