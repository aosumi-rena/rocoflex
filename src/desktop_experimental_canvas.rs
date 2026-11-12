//! Experimental canvas controls and rendering.
use super::*;
use super::experimental_math::{self as math, V3};
use std::sync::Arc;

#[derive(Clone)]
struct Scene3 { sources: Vec<math::MixSource>, has_depth: bool }
struct AxisDrag {
    axis: usize, press: [f64;2], origin: V3, rect: Rect, camera: math::Camera,
    undo: Option<transactions::UndoEntry>,
}

const MARKER_RADIUS: f64 = 0.020;
const MARKER_OUTLINE: f32 = 0.115;
const TRAJECTORY_WIDTH: f64 = 0.006;

#[derive(Clone, Copy)]
struct SourcePick { owner: usize, point: usize, segment_at: Option<f32> }

#[derive(Clone, Copy)]
struct CanvasProjection { camera: math::Camera, rect: Rect, size: [f64;2] }

#[derive(Clone, Copy)]
struct SourceMark { a: Pos2, b: Option<Pos2>, depth: f64, size: f32 }

impl CanvasProjection {
    fn new(camera: math::Camera, rect: Rect) -> Self {
        Self { camera, rect, size: [rect.width() as f64, rect.height() as f64] }
    }
    fn screen(self, projected: V3) -> Option<Pos2> {
        let at=self.rect.min+vec2(projected[0] as f32,projected[1] as f32);
        at.is_finite().then_some(at)
    }
    fn project(self, point: V3) -> Option<(Pos2, f64)> {
        let v=self.camera.project(point,self.size)?;
        Some((self.screen(v)?,v[2]))
    }
    fn marker(self, point: V3) -> Option<SourceMark> {
        let (a,depth)=self.project(point)?;
        self.marker_at(a,depth)
    }
    fn marker_at(self, a: Pos2, depth: f64) -> Option<SourceMark> {
        let size=(self.camera.pixels_per_unit(depth,self.size)?*MARKER_RADIUS) as f32;
        self.rect.expand(size*(1.0+MARKER_OUTLINE)).contains(a)
            .then_some(SourceMark {a,b:None,depth,size})
    }
    fn segment(self, a: V3, b: V3) -> Option<SourceMark> {
        let [a,b]=self.camera.project_segment(a,b,self.size)?;
        let depth=(a[2]+b[2])*0.5;
        let size=(self.camera.pixels_per_unit(depth,self.size)?*TRAJECTORY_WIDTH) as f32;
        Some(SourceMark {a:self.screen(a)?,b:Some(self.screen(b)?),depth,size})
    }
}

impl SourceMark {
    fn pick(self, at: Pos2) -> Option<f32> {
        if let Some(b)=self.b {
            let delta=b-self.a;
            let length=delta.length_sq();
            let t=if length.is_finite() && length>0.0 {
                ((at-self.a).dot(delta)/length).clamp(0.0,1.0)
            } else {0.0};
            (self.a.lerp(b,t).distance(at)<=self.size*0.5+3.0).then_some(t)
        } else {
            (self.a.distance(at)<=self.size*(1.0+MARKER_OUTLINE)+2.0).then_some(0.0)
        }
    }
}

pub(super) struct State {
    pub enabled: bool,
    pub fixed_scale: bool,
    pub mover_enabled: bool,
    pub scale_percent: f64,
    pub z: f64,
    depth_anchor: Option<[f64;4]>,
    pub camera: math::Camera,
    pub speed: f64,
    mover: V3,
    mover_origin: V3,
    mover_editing: bool,
    axis_drag: Option<AxisDrag>,
    hovered: Option<usize>,
    source_pick: Option<SourcePick>,
    hint: Option<(String, Instant)>,
    pub canvas_id: Option<egui::Id>,
    key_gate: math::KeyGate,
    shift_held: bool,
    shift_owned: bool,
    navigation_batch_allowed: bool,
    geometry_epoch: u64,
    scene: Option<(u64, Arc<Scene3>)>,
    mix: Option<([u64;5], Arc<crate::prototype_controls::ControllerMix>)>,
}
impl Default for State {
    fn default() -> Self { Self { enabled:false, fixed_scale:false, mover_enabled:false, scale_percent:100.0,
        z:0.0, depth_anchor:None, camera:Default::default(), speed:0.8, mover:[0.0;3],
        mover_origin:[0.0;3], mover_editing:false, axis_drag:None,
        hovered:None, source_pick:None, hint:None, canvas_id:None, key_gate:Default::default(), shift_held:false, shift_owned:false,
        navigation_batch_allowed:true, geometry_epoch:0, scene:None, mix:None } }
}

impl State {
    pub(super) fn has_axis_drag(&self)->bool {self.axis_drag.is_some()}
    pub(super) fn retire_pointer(&mut self) { self.axis_drag=None; self.hovered=None; }
    pub(super) fn retire_input(&mut self) {self.retire_pointer();self.key_gate.reset();self.shift_owned=false;}
    pub(super) fn observe_shift(&mut self, held:bool, admitted:bool) {
        if !held || !admitted || !self.navigation_batch_allowed {self.shift_owned=false;}
        else if !self.shift_held {self.shift_owned=true;}
        self.shift_held=held;
    }
    pub(super) fn invalidate_geometry(&mut self) {
        self.geometry_epoch = self.geometry_epoch.wrapping_add(1);
        self.scene = None; self.mix = None; self.source_pick = None;
    }
}
impl DesktopApp {
    pub(crate) fn set_experimental_navigation_batch(&mut self,allowed:bool) {self.experimental.navigation_batch_allowed=allowed;}
    /// Also consumed by keyboard binding dispatch. Modal and text focus always win.
    pub(super) fn experimental_canvas_navigation_active(&self, ctx: &egui::Context) -> bool {
        self.experimental.enabled && !self.layout_blocked() && !self.settings
            && self.editor.is_none() && !self.color_picker && self.error.is_none()
            && self.notice.is_none() && self.menu.is_none() && !ctx.text_edit_focused()
            && ctx.memory(|m|m.focused().is_none_or(|id|Some(id)==self.experimental.canvas_id))
            && !egui::Popup::is_any_open(ctx) && ctx.input(|i| i.raw.focused)
    }
    fn experimental_hint(&mut self, text: impl Into<String>) {
        self.experimental.hint=Some((text.into(),Instant::now()));
    }
    fn reset_probe_mover(&mut self) {
        let position=[self.probe.position.x,self.probe.position.y,self.experimental.z];
        self.experimental.mover=position;
        self.experimental.mover_origin=position;
        self.experimental.mover_editing=false;
    }
    pub(super) fn experimental_restore_document(&mut self) {
        let metadata=crate::preset::experimental_canvas_metadata(&self.preset.document)
            .expect("Preset::parse validated experimental canvas metadata");
        self.experimental.enabled=metadata.is_some_and(|m|m.enabled);
        self.experimental.z=metadata.map_or(0.0,|m|m.probe_z);
        self.experimental.depth_anchor=metadata.and_then(|m|m.depth_axis);
        self.experimental.fixed_scale=metadata.is_some_and(|m|m.fixed_scale.is_some());
        self.experimental.scale_percent=metadata.and_then(|m|m.fixed_scale).unwrap_or(100.0);
        self.experimental.mover_enabled=metadata.is_some_and(|m|m.mover_enabled);
        self.reset_probe_mover();
        self.experimental.retire_input(); self.experimental.scene=None; self.experimental.mix=None;
    }
    pub(super) fn experimental_store_document(&mut self) {
        if !self.experimental_uses_preset_features() {
            if let Some(doc)=self.preset.document.as_object_mut() {doc.remove("rocoflexExperimentalCanvas");}
            return;
        }
        let waypoint_z = self.preset.document.get_mut("rocoflexExperimentalCanvas")
            .and_then(Value::as_object_mut).and_then(|metadata| metadata.remove("waypointZ"));
        let mut metadata=json!({
            "version":1,"enabled3d":self.experimental.enabled,"probeZ":self.experimental.z,
            "depthAxis":self.experimental.depth_anchor,
            "fixedScale":self.experimental.fixed_scale.then_some(self.experimental.scale_percent),
            "probeMover":self.experimental.mover_enabled,
        });
        if let Some(depths) = waypoint_z { metadata["waypointZ"] = depths; }
        self.preset.document["rocoflexExperimentalCanvas"]=metadata;
    }
    pub(super) fn experimental_uses_preset_features(&self)->bool {
        self.experimental.enabled || self.experimental.fixed_scale || self.experimental.mover_enabled
    }
    pub(super) fn experimental_preset_output(&self)->bool {
        self.experimental_uses_preset_features() || self.preset.document.get("rocoflexExperimentalCanvas").is_some()
    }
    fn experimental_scene(&mut self) -> std::result::Result<Arc<Scene3>,String> {
        if let Some((revision,scene))=&self.experimental.scene {
            if *revision==self.experimental.geometry_epoch {return Ok(scene.clone());}
        }
        let mut raw=Vec::new(); let mut reduced=Vec::new();
        for (s,source) in self.preset.sources.iter().enumerate() {
            for (i,point) in source.points.iter().enumerate() {
                let row: [f32;4]=point.reduced.as_slice().try_into().map_err(|_|"Invalid reduced source dimensions")?;
                reduced.push(row);
                let xy=if self.native_layout {
                    *self.layout_view.as_ref().and_then(|v|v.positions.get(s)).and_then(|v|v.get(i))
                        .ok_or("Canvas/source shape mismatch")?
                } else if self.mapping==2 { self.preset.basis_points[s][i] }
                else { [row[0],row[1]] };
                raw.push(xy);
            }
        }
        if raw.iter().flatten().any(|x|!x.is_finite()) {return Err("Invalid 2D canvas geometry".into());}
        let bounds=if raw.is_empty(){([0.0_f64;2],[0.0_f64;2])} else {
            let mut lo=[f64::INFINITY;2];let mut hi=[f64::NEG_INFINITY;2];
            for p in &raw {for i in 0..2 {lo[i]=lo[i].min(p[i] as f64);hi[i]=hi[i].max(p[i] as f64);}} (lo,hi)
        };
        let center=[(bounds.0[0]+bounds.1[0])*0.5,(bounds.0[1]+bounds.1[1])*0.5];
        let scale=(2.5/(bounds.1[0]-bounds.0[0]).max(0.01)).min(1.7/(bounds.1[1]-bounds.0[1]).max(0.01));
        let basis=if self.mapping==1 && !self.native_layout {[1.,0.,0.,0.,0.,1.,0.,0.]}
            else if self.native_layout {self.layout.working_uv_snapshot().unwrap_or(self.preset.transformation)}
            else {self.preset.transformation};
        let previous_anchor=self.experimental.depth_anchor;
        let (z,has_depth,anchor)=math::depth(&reduced,basis,previous_anchor).map_err(str::to_owned)?;
        // Rendering/restore never edits the document. An initially selected or
        // replacement anchor stays transient until an explicit user action.
        if anchor!=previous_anchor {self.experimental.depth_anchor=anchor;}
        let mut cursor=0; let mut sources=Vec::new();
        for (owner,source) in self.preset.sources.iter().enumerate() {
            let offset=self.offsets.get(owner).copied().unwrap_or_default();
            let mut points=Vec::new();
            for point in &source.points {
                let p=raw[cursor];
                let xy=if self.native_layout {[p[0] as f64+offset.x,p[1] as f64+offset.y]}
                    else {[(p[0] as f64-center[0])*scale+offset.x,(p[1] as f64-center[1])*scale+offset.y]};
                points.push(([xy[0],xy[1],z[cursor]],point.enabled)); cursor+=1;
            }
            sources.push(math::MixSource {trajectory:source.kind=="trajectory",enabled:source.enabled,points});
        }
        let scene=Arc::new(Scene3{sources,has_depth}); self.experimental.scene=Some((self.experimental.geometry_epoch,scene.clone())); Ok(scene)
    }
    pub(super) fn experimental_mix(&mut self) -> std::result::Result<Arc<crate::prototype_controls::ControllerMix>,String> {
        let p=[self.probe.position.x,self.probe.position.y,self.experimental.z];
        let key=[self.revision,p[0].to_bits(),p[1].to_bits(),p[2].to_bits(),self.probe.radius.to_bits()];
        if let Some((k,mix))=&self.experimental.mix {if *k==key{return Ok(mix.clone());}}
        let scene=self.experimental_scene()?;
        let weights=math::weights(&scene.sources,p,self.probe.radius).map_err(str::to_owned)?;
        let mut point_weights=Vec::new(); let mut blends=Vec::new();
        for (source,values) in scene.sources.iter().zip(weights) {
            let w:Vec<[f32;2]>=values.into_iter().map(|v|[v as f32,0.0]).collect();
            let blend=if source.trajectory {Some(crate::prototype_controls::trajectory_blend(&w)?)} else {None};
            point_weights.extend_from_slice(&w); blends.push(blend);
        }
        let mix=Arc::new(crate::prototype_controls::ControllerMix {point_weights,positive_scale:1.0,
            negative_scale:0.0,trajectory_blends:blends});
        self.experimental.mix=Some((key,mix.clone())); Ok(mix)
    }
    fn experimental_set_probe(&mut self, value: V3, remember: bool) -> bool {
        if value.iter().any(|v|!v.is_finite() || !(-2.0..=2.0).contains(v)) {return false;}
        self.experimental_update_probe(value, remember)
    }
    pub(super) fn experimental_recall_waypoint(&mut self, value: V3) -> bool {
        if self.layout_blocked() { return false; }
        if value == [self.probe.position.x, self.probe.position.y, self.experimental.z]
            && value[2].to_bits() == self.experimental.z.to_bits()
        {
            return self.probe_changed(Probe { position: Point::new(value[0], value[1]), ..self.probe });
        }
        // Legacy presets may have XY coordinates outside the 3D navigation box.
        self.experimental_update_probe(value, false)
    }
    fn experimental_update_probe(&mut self, value: V3, remember: bool) -> bool {
        let xy_changed=value[0].to_bits()!=self.probe.position.x.to_bits()
            || value[1].to_bits()!=self.probe.position.y.to_bits();
        let z_changed=self.experimental.enabled && value[2].to_bits()!=self.experimental.z.to_bits();
        if !xy_changed && !z_changed {return true;}
        let previous=remember.then(||self.capture_undo());
        // Z-only motion must not restart the 2D controller's reload loop.
        if xy_changed && !self.probe_changed(Probe {position:Point::new(value[0],value[1]),..self.probe}) {return false;}
        self.experimental.z=value[2];
        if self.experimental.enabled {
            self.experimental_store_document(); self.experimental.mix=None;
            if z_changed {self.live.set_restored_canvas(None);}
        }
        if !xy_changed {self.changed();}
        if let Some(previous)=previous {self.remember_entry(previous);}
        true
    }
    pub(super) fn experimental_features_ui(&mut self, ui:&mut egui::Ui) {
        let first=ui.ctx().current_pass_index()==0;
        let width=ui.available_width().min(492.0);
        controls::settings_section(ui,width,"Experimental Features");
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y=0.0;
            let mut fixed=self.experimental.fixed_scale;
            if experimental_check_row(ui,width,"experimental-rescale","Canvas Rescale",&mut fixed,first) {
                self.remember();
                self.experimental.fixed_scale=fixed;
                self.experimental_store_document(); self.changed();
            }
            let mut scale=self.experimental.scale_percent;
            ui.add_enabled_ui(self.experimental.fixed_scale,|ui| {
                let row=controls::settings_row(ui,width,"Fixed canvas scale");
                let rect=controls::settings_value_rect(row);
                let mut child=ui.new_child(egui::UiBuilder::new().max_rect(rect));
                child.add_sized(rect.size(),egui::DragValue::new(&mut scale)
                    .range(10.0..=500.0).speed(1.0).suffix("%"));
            });
            if first && self.experimental.fixed_scale && scale.is_finite()
                && (10.0..=500.0).contains(&scale) && scale!=self.experimental.scale_percent {
                self.experimental.scale_percent=scale;
                self.experimental_store_document(); self.changed();
            }
            ui.label(egui::RichText::new(localization::tr(ui.ctx(),
                "100% uses the canvas unit scale; the probe no longer triggers automatic zoom.")).size(12.0).color(MUTED));
            let mut enabled=self.experimental.enabled;
            if experimental_check_row(ui,width,"experimental-3d","3D Canvas",&mut enabled,first) {
                self.remember(); self.experimental.enabled=enabled; self.experimental.retire_input();
                self.reset_probe_mover();
                if enabled {let _=self.experimental_scene();}
                self.experimental_store_document(); self.live.set_restored_canvas(None); self.experimental.mix=None;
                self.changed();
                if enabled { self.experimental_hint(localization::tr(ui.ctx(),
                    "WASD / arrows: move · Space / Shift: up / down · middle drag: rotate")); }
            }
            if self.experimental.enabled {
                ui.label(egui::RichText::new(localization::tr(ui.ctx(),
                    "3D positioning changes how voices blend.")).size(12.0).color(MUTED));
            }
            ui.add_space(7.0);
            let mut mover_enabled=self.experimental.mover_enabled;
            if experimental_check_row(ui,width,"experimental-mover","Probe Mover",&mut mover_enabled,first) {
                self.remember();
                self.experimental.mover_enabled=mover_enabled;
                if mover_enabled {self.reset_probe_mover();}
                self.experimental_store_document(); self.changed();
            }
        });
    }

    pub(super) fn probe_mover_ui(&mut self, ctx: &egui::Context, full: Rect) {
        let first=ctx.current_pass_index()==0;
        if !self.experimental.mover_enabled || self.settings || self.editor.is_some()
            || self.error.is_some() || self.notice.is_some() || self.color_picker || self.menu.is_some()
            || egui::Popup::is_any_open(ctx) {
            if first {self.experimental.mover_editing=false;}
            return;
        }
        let position=[self.probe.position.x,self.probe.position.y,self.experimental.z];
        if first && !self.experimental.mover_editing && position!=self.experimental.mover_origin {
            self.reset_probe_mover();
        }
        let mut coordinates=self.experimental.mover;
        let mut editing=false;
        let count=if self.experimental.enabled {3} else {2};
        let window=egui::Window::new(egui::RichText::new(localization::tr(ctx,"Probe Mover")).size(14.0))
            .id(egui::Id::new("probe-mover")).order(egui::Order::Foreground)
            .default_pos(full.left_bottom()+vec2(58.0,-158.0)).default_size(vec2(168.0,110.0))
            .resizable(false).movable(true).collapsible(true).constrain_to(full)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y=4.0;
                ui.spacing_mut().interact_size.y=23.0;
                ui.style_mut().text_styles.insert(egui::TextStyle::Button,FontId::proportional(13.0));
                for (axis,value) in ["X","Y","Z"].into_iter().zip(coordinates.iter_mut()).take(count) {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(axis).size(13.0));
                        let response=ui.add(egui::DragValue::new(value).range(-2.0..=2.0).speed(0.005).max_decimals(9));
                        editing|=response.has_focus() || response.is_pointer_button_down_on() || response.dragged();
                    });
                }
                let (button,_)=ui.allocate_exact_size(vec2(ui.available_width(),24.0),Sense::hover());
                controls::settings_button(ui,button,"probe-mover-move","Move",true).clicked()
            });
        if let Some(window)=window {
            ctx.move_to_top(window.response.layer_id);
            self.overlay_rects.push(window.response.rect);
            if first {
                self.experimental.mover=coordinates;
                self.experimental.mover_editing=editing;
                if window.inner==Some(true) {
                    if self.experimental_set_probe(coordinates,true) {self.reset_probe_mover();}
                    else {self.layout_status=Some(localization::tr(ctx,"Invalid probe coordinates.").into());}
                } else if !editing && position!=self.experimental.mover_origin {
                    // The coordinate editor just released ownership of the draft.
                    self.reset_probe_mover();
                    ctx.request_repaint();
                }
            }
        }
    }

    fn pick_experimental_source(&mut self, at: Pos2, full: Rect) {
        let projection=CanvasProjection::new(self.experimental_camera(),full);
        let Ok(scene)=self.experimental_scene() else {return;};
        let mut hit: Option<(f64,SourcePick)>=None;
        for (owner,source) in scene.sources.iter().enumerate().filter(|(_,s)|s.enabled) {
            for (point,&(world,enabled)) in source.points.iter().enumerate() {
                if !enabled {continue;}
                let segment=source.trajectory.then(||source.points.get(point+1)).flatten()
                    .filter(|(_,enabled)|*enabled)
                    .and_then(|(next,_)|projection.segment(world,*next));
                for mark in [projection.marker(world),segment].into_iter().flatten() {
                    if let Some(t)=mark.pick(at)
                        && hit.is_none_or(|(depth,_)|mark.depth<=depth) {
                        hit=Some((mark.depth,SourcePick {owner,point,segment_at:mark.b.map(|_|t)}));
                    }
                }
            }
        }
        self.experimental.source_pick=hit.map(|(_,pick)|pick);
        self.selected=self.experimental.source_pick.map(|pick|pick.owner);
        self.selected_at=at;
        self.source_menu_cache=None;
    }

    pub(super) fn experimental_source_actions_rect(&mut self, owner: usize, full: Rect, size: egui::Vec2) -> Option<Rect> {
        let projection=CanvasProjection::new(self.experimental_camera(),full);
        let scene=self.experimental_scene().ok()?;
        let source=scene.sources.get(owner)?;
        let occupied: Vec<_>=source.points.iter().filter(|(_,enabled)|*enabled)
            .filter_map(|(p,_)|projection.marker(*p).map(|mark|mark.a)).collect();
        let origin=self.experimental.source_pick.filter(|pick|pick.owner==owner).and_then(|pick| {
            let (world,enabled)=*source.points.get(pick.point)?;
            if !enabled {return None;}
            if let Some(t)=pick.segment_at {
                let (next,enabled)=*source.points.get(pick.point+1)?;
                if !source.trajectory || !enabled {return None;}
                let mark=projection.segment(world,next)?;
                Some(mark.a.lerp(mark.b?,t))
            } else {projection.marker(world).map(|mark|mark.a)}
        }).filter(|at|at.is_finite()).or_else(||occupied.first().copied())?;
        Some(source_menu_position(origin,full,&occupied,size))
    }

    pub(super) fn experimental_input(&mut self, ctx:&egui::Context, rect:Rect, response:&egui::Response,
        cancellations:&[canvas_gesture::PointerCancellation], escape_aborts:bool) {
        if ctx.current_pass_index()!=0 {return;}
        if escape_aborts || cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary)) {
            self.experimental.axis_drag=None; self.waypoint_cancel(); return;
        }
        if !self.experimental_canvas_navigation_active(ctx) {self.experimental.retire_input();return;}
        if self.experimental.axis_drag.is_none() && self.waypoint_primary(ctx, rect, response, cancellations) {
            self.experimental.hovered = None;
            ctx.request_repaint();
            return;
        }
        let mut acted=false;
        let axis_owned=self.experimental.axis_drag.is_some();
        let mut axis_claimed=axis_owned;
        let axis_terminal=axis_owned && ctx.input(|i|i.raw.events.iter().any(|event|
            matches!(event,egui::Event::PointerButton{button:egui::PointerButton::Primary,..})));
        let keys=[egui::Key::W,egui::Key::A,egui::Key::S,egui::Key::D,
            egui::Key::ArrowUp,egui::Key::ArrowLeft,egui::Key::ArrowDown,egui::Key::ArrowRight,
            egui::Key::Plus,egui::Key::Equals,egui::Key::Minus,egui::Key::Space];
        let (down,fresh,disqualified,signs,dt,modifiers)=ctx.input(|i| {
            let mut down=0u16;let mut fresh=0u16;let mut disqualified=0u16;let mut signs=Vec::new();
            for (bit,key) in keys.iter().enumerate() {
                if i.key_down(*key) {down|=1<<bit;}
                for event in &i.raw.events {
                    if let egui::Event::Key {key:k,pressed:true,repeat,modifiers,..}=event && k==key {
                        let allowed=if bit<8 || bit==11 {!modifiers.any()}
                            else {!modifiers.alt && !modifiers.mac_cmd && (modifiers.ctrl || modifiers.shift)};
                        if allowed && !repeat {fresh|=1<<bit;}
                        if !allowed {disqualified|=1<<bit;}
                        if (8..11).contains(&bit) {signs.push(math::SignEvent {bit:1<<bit,increase:bit!=10,
                            ctrl:modifiers.ctrl,shift:modifiers.shift,alt:modifiers.alt,mac_cmd:modifiers.mac_cmd});}
                    }
                }
            }
            (down,fresh,disqualified,signs,i.stable_dt as f64,i.modifiers)
        });
        let permit=if self.experimental.navigation_batch_allowed {self.experimental.key_gate.permit(down,fresh,disqualified)}
            else {self.experimental.key_gate.reset();0};
        let held=down & permit;
        let has=|a:u16,b:u16|held & (a|b)!=0;
        let forward=(has(1<<0,1<<4) as i8-has(1<<2,1<<6) as i8) as f64;
        let right=(has(1<<3,1<<7) as i8-has(1<<1,1<<5) as i8) as f64;
        let ctrl=modifiers.ctrl; let shift=modifiers.shift;
        let sign=math::sign_action(&signs).filter(|(bit,_)| permit & bit!=0);
        let shift_chord=ctx.input(|i| i.key_down(egui::Key::Plus) || i.key_down(egui::Key::Equals)
            || i.key_down(egui::Key::Minus));
        if shift && (shift_chord || sign.is_some()) {self.experimental.shift_owned=false;}
        let vertical=if shift && self.experimental.shift_owned {-1.0}
            else if !shift && held & (1<<11)!=0 {1.0} else {0.0};
        if !axis_owned && self.experimental.navigation_batch_allowed && !modifiers.alt && !ctrl && !modifiers.mac_cmd
            && ((!shift && (forward!=0.0 || right!=0.0)) || vertical!=0.0) {
            self.experimental.camera.walk_vertical(if shift {0.0} else {forward},if shift {0.0} else {right},vertical,dt,self.experimental.speed);
            acted=true;
            self.experimental_hint(localization::tr(ctx,if vertical>0.0 {"Move up"} else if vertical<0.0 {"Move down"} else {"WASD / arrows: move POV"}));
        }
        if !axis_owned && let Some((_,action))=sign {
            match action {
                math::SignAction::Zoom(increase)=>{
                    let factor=if increase {1.1} else {1.0/1.1};
                    self.experimental.camera.zoom=(self.experimental.camera.zoom*factor).clamp(0.1,5.0);
                    let zoom=self.experimental.camera.zoom*100.0;
                    self.experimental_hint(localization::input_value(ctx,"POV scale {value}%",&format!("{zoom:.0}")));acted=true;
                }
                math::SignAction::Speed(increase)=>{
                    let factor=if increase {1.1} else {1.0/1.1};
                    self.experimental.speed=(self.experimental.speed*factor).clamp(0.05,10.0);
                    self.experimental_hint(localization::input_value(ctx,"Movement step {value}",&format!("{:.2}",self.experimental.speed)));acted=true;
                }
            }
        }
        let pointer=ctx.pointer_hover_pos().filter(|p|p.x.is_finite() && p.y.is_finite());
        let over=pointer.is_some_and(|p|self.overlay_rects.iter().any(|r|r.contains(p)));
        if !axis_owned && !over && response.hovered() {self.resize_probe_from_wheel(ctx);}
        if !axis_owned && !over && !cancellations.iter().any(|c|c.affects(egui::PointerButton::Middle))
            && response.dragged_by(egui::PointerButton::Middle) {
            let d=ctx.input(|i|i.pointer.delta()); self.experimental.camera.rotate([d.x as f64,d.y as f64]);
            self.experimental_hint(localization::tr(ctx,"Middle drag: rotate POV"));acted=true;
        }
        let size=[rect.width() as f64,rect.height() as f64];
        let camera=self.experimental_camera();
        let current=[self.probe.position.x,self.probe.position.y,self.experimental.z];
        let local=|p:Pos2|[f64::from(p.x-rect.left()),f64::from(p.y-rect.top())];
        let admits=|p:Pos2, overlays:&[Rect]|p.x.is_finite() && p.y.is_finite() && rect.contains(p)
            && !overlays.iter().any(|r|r.contains(p))
            && ctx.layer_id_at(p).is_none_or(|layer|layer==response.layer_id);
        self.experimental.hovered=pointer.filter(|p|admits(*p,&self.overlay_rects)).and_then(|p|hit_axis(current,camera,size,local(p)));
        let down=ctx.input(|i|i.pointer.primary_down());
        // A complete press/move/release can arrive in one input frame. egui
        // does not report it as a drag, so use the checked raw event sequence.
        let completed=if self.experimental.axis_drag.is_none() && !down {
            ctx.input(|i|canvas_gesture::completed_primary_drag(&i.raw.events,cancellations))
                .filter(|gesture|admits(gesture.press,&self.overlay_rects))
        } else {None};
        if let Some(gesture)=completed {
            if let Some(axis)=hit_axis(current,camera,size,local(gesture.press)) {
                axis_claimed=true;
                if let Some(mut next)=math::drag_axis(current,axis,local(gesture.press),local(gesture.release),camera,size) {
                    for v in &mut next {*v=v.clamp(-2.0,2.0);}
                    if next!=current && self.experimental_set_probe(next,true) {acted=true;}
                }
            }
        } else if self.experimental.axis_drag.is_none() && down && response.is_pointer_button_down_on() {
            // Accept one actual down in this batch, never a later hover position
            // or an ambiguous release/repress sequence.
            let press=ctx.input(|i| {
                let mut found=None;
                for event in &i.raw.events {
                    match event {
                        egui::Event::WindowFocused(false) | egui::Event::PointerGone => return None,
                        egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed,..} => {
                            if !pressed || found.is_some() {return None;} found=Some(*pos);
                        }
                        _=>{}
                    }
                }
                found
            }).filter(|p|admits(*p,&self.overlay_rects));
            if let Some(press)=press {
                if let Some(axis)=hit_axis(current,camera,size,local(press)) {
                    axis_claimed=true;
                    self.experimental.axis_drag=Some(AxisDrag {axis,press:local(press),origin:current,
                        rect,camera,undo:Some(self.capture_undo())});
                }
            }
        }
        if let Some(mut drag)=self.experimental.axis_drag.take() {
            // A resize changes the projection; retire the old owner instead of
            // applying a stale pointer transform to the new viewport.
            if drag.rect==rect {
                let at=if down && !axis_terminal {pointer} else {
                    ctx.input(|i|canvas_gesture::armed_primary_release(&i.raw.events,cancellations))
                };
                if let Some(at)=at {
                    if let Some(mut next)=math::drag_axis(drag.origin,drag.axis,drag.press,local(at),drag.camera,size) {
                        for v in &mut next {*v=v.clamp(-2.0,2.0);}
                        if next!=current && self.experimental_set_probe(next,false) {
                            if let Some(previous)=drag.undo.take(){self.remember_entry(previous);}
                            acted=true;
                        }
                    }
                }
                if down && !axis_terminal {self.experimental.axis_drag=Some(drag);}
            }
        }
        if !axis_claimed && !down && !response.dragged_by(egui::PointerButton::Middle) {
            let clicked=response.clicked_by(egui::PointerButton::Primary);
            let click=ctx.input(|i| {
                if let Some(up)=canvas_gesture::primary_click_release(&i.raw.events,cancellations) {
                    if !clicked && completed.is_some() {return None;}
                    let press=i.raw.events.iter().find_map(|event|match event {
                        egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed:true,..}=>Some(*pos),
                        _=>None,
                    })?;
                    Some((Some(press),up))
                } else if clicked {
                    canvas_gesture::armed_primary_release(&i.raw.events,cancellations).map(|up|(None,up))
                } else {None}
            });
            let admitted=|p|admits(p,&self.overlay_rects);
            if let Some((_,up))=click.filter(|(press,up)|press.is_none_or(admitted) && admitted(*up)) {
                self.pick_experimental_source(up,rect);
                acted=true;
            }
        }
        if acted {ctx.request_repaint();}
    }

    pub(super) fn experimental_camera(&self)->math::Camera {
        let mut c=self.experimental.camera;
        if self.experimental.fixed_scale {c.zoom*=self.experimental.scale_percent.clamp(10.0,500.0)/100.0;}
        c
    }
    pub(super) fn paint_experimental_canvas(&mut self, ui:&egui::Ui, rect:Rect) {
        let painter=ui.painter().with_clip_rect(rect); painter.rect_filled(rect,0.0,Color32::from_rgb(26,28,35));
        let camera=self.experimental_camera(); let size=[rect.width() as f64,rect.height() as f64];
        let projection=CanvasProjection::new(camera,rect);
        let screen=|v|projection.screen(v);
        let project=|p|projection.project(p);
        for axis in 0..3 {
            let u=(axis+1)%3; let v=(axis+2)%3;
            let t=(camera.position[axis]*2.0+0.5).clamp(0.0,1.0) as f32;
            let fade=t*t*(3.0-2.0*t);
            for (side,alpha) in [(-2.0,fade),(2.0,1.0-fade)] {
                if alpha==0.0 {continue;}
                for i in -4_i32..=4 {
                    for (across,along) in [(u,v),(v,u)] {
                        let mut a=[0.0;3]; a[axis]=side; a[across]=i as f64*0.5; a[along]=-2.0;
                        let mut b=a; b[along]=2.0;
                        if let Some([a,b])=camera.project_segment(a,b,size)
                            && let (Some(a),Some(b))=(screen(a),screen(b)) {
                            let opacity=if i.abs()==4 {49.0} else {17.0};
                            let color=Color32::from_white_alpha((opacity*alpha).round() as u8);
                            painter.line_segment([a,b],egui::Stroke::new(1.0,color));
                        }
                    }
                }
            }
        }
        let scene=match self.experimental_scene() {Ok(scene)=>scene,Err(_)=>{
            painter.text(rect.center(),Align2::CENTER_CENTER,localization::tr(ui.ctx(), "Could not display the 3D canvas."),FontId::proportional(13.0),Color32::LIGHT_RED);return;
        }};
        let mut marks:Vec<(SourceMark,Color32,bool)>=Vec::new();
        let mut captions=vec![Vec::new();scene.sources.len()];
        let probe=[self.probe.position.x,self.probe.position.y,self.experimental.z];
        let probe_screen=project(probe).map(|(at,_)|at);
        let mix=self.experimental_mix().ok();
        let mut weights=mix.as_ref().map(|m|m.point_weights.iter());
        let mut probe_tint=math::ProbeTint::default();
        for (owner,source) in scene.sources.iter().enumerate() {
            let [_,r,g,b]=paint::parse_argb(&self.preset.sources[owner].color).to_be_bytes();
            let rgb=[r,g,b];
            let color=Color32::from_rgb(rgb[0],rgb[1],rgb[2]);
            let selected=self.selected==Some(owner);
            let mut center=[0.0;3]; let mut strength=0.0_f64;
            for (index,(p,enabled)) in source.points.iter().enumerate() {
                let weight=weights.as_mut().and_then(Iterator::next).map_or(0.0,|w|f64::from(w[0]));
                if !source.enabled || !enabled {continue;}
                strength+=weight;
                for i in 0..3 {center[i]+=p[i]*weight;}
                if let Some((at,depth))=project(*p) {
                    captions[owner].push(at);
                    if let Some(mark)=projection.marker_at(at,depth) {
                        marks.push((mark,color,selected));
                    }
                    if weight>=0.01 && let Some(to)=probe_screen {
                        let opacity=(weight*8.0).min(1.0).sqrt() as f32;
                        painter.line_segment([at,to],egui::Stroke::new(3.0,color.gamma_multiply(0.20*opacity)));
                        painter.line_segment([at,to],egui::Stroke::new(1.2,color.gamma_multiply(0.80*opacity)));
                    }
                }
                if source.trajectory && let Some(&(next,true))=source.points.get(index+1)
                    && let Some(mark)=projection.segment(*p,next) {
                    marks.push((mark,color,selected));
                }
            }
            probe_tint.add(rgb,strength);
            if strength>0.0 && let Some((from,_))=project(center.map(|n|n/strength))
                && let Some(to)=probe_screen {
                let alpha=strength.min(1.0).sqrt() as f32;
                glow(&painter,from,rect.width().min(rect.height())*0.24,rgb,0.13*f64::from(alpha));
                painter.line_segment([from,to],egui::Stroke::new(5.0,color.gamma_multiply(0.14*alpha)));
                painter.line_segment([from,to],egui::Stroke::new(1.5,color.gamma_multiply(0.72*alpha)));
            }
        }
        marks.sort_by(|a,b|b.0.depth.total_cmp(&a.0.depth));
        for (m,color,selected) in marks {
            if let Some(b)=m.b {painter.line_segment([m.a,b],egui::Stroke::new(m.size,color));}
            else {
                painter.circle_filled(m.a,m.size,color);
                if selected {painter.circle_filled(m.a,m.size,WASH);}
                painter.circle_stroke(m.a,m.size,egui::Stroke::new(m.size*MARKER_OUTLINE,Color32::WHITE));
            }
        }
        self.paint_projected_source_labels(&painter,rect,&captions);
        let p=probe;
        if let Some((at,depth))=project(p) {
            let r=(self.probe.radius*size[0].min(size[1])*1.4*camera.zoom/depth).clamp(5.0,100.0) as f32;
            paint_probe_sphere(&painter,at,r,probe_tint.color());
            painter.circle_stroke(at,r,egui::Stroke::new(1.5,Color32::WHITE));
            let sphere_hovered=self.experimental_canvas_navigation_active(ui.ctx())
                && ui.ctx().pointer_hover_pos().is_some_and(|pointer| rect.contains(pointer)
                    && pointer.distance(at)<=r+6.0 && !self.overlay_rects.iter().any(|overlay|overlay.contains(pointer)));
            for (axis,color) in [(0,Color32::from_rgb(245,95,90)),(1,Color32::from_rgb(100,220,120)),(2,Color32::from_rgb(100,160,255))] {
                let active=self.experimental.hovered==Some(axis)
                    || self.experimental.axis_drag.as_ref().is_some_and(|d|d.axis==axis);
                if !active && !sphere_hovered {continue;}
                if let Some([_,_,tip])=camera.axis_projection(p,axis,size)
                    && let Some(end)=screen(tip) {
                    painter.line_segment([at,end],egui::Stroke::new(if active {4.0}else{2.5},color));
                    painter.circle_filled(end,if active {5.0}else{3.5},color);
                    painter.text(end,Align2::CENTER_BOTTOM,["X","Y","Z"][axis],FontId::proportional(11.0),color);
                }
            }
        }
        if !scene.has_depth && !scene.sources.is_empty() {painter.text(rect.left_bottom()+vec2(14.0,-64.0),Align2::LEFT_BOTTOM,
            localization::tr(ui.ctx(),"No independent depth in this preset"),FontId::proportional(12.0),Color32::LIGHT_GRAY);}
        if let Some((text,when))=&self.experimental.hint {
            let elapsed=when.elapsed().as_secs_f32();
            if elapsed<2.4 {let alpha=((2.4-elapsed)/0.6).clamp(0.0,1.0);
                painter.text(rect.center_top()+vec2(0.0,75.0-elapsed*7.0),Align2::CENTER_TOP,text,FontId::proportional(13.0),Color32::from_white_alpha((alpha*230.0) as u8));
                ui.ctx().request_repaint_after(Duration::from_millis(33));}
        }
    }
}
fn paint_probe_sphere(painter:&egui::Painter, center:Pos2, radius:f32, tint:[u8;3]) {
    const RINGS: u32 = 8;
    const SLICES: u32 = 32;
    let mut mesh=egui::Mesh::default();
    let shade=|x:f32,y:f32,z:f32| {
        let diffuse=(-0.35*x-0.43*y+0.83*z).max(0.0);
        let light=0.64+0.36*diffuse;
        let glint=0.32*diffuse.powi(16);
        let [r,g,b]=tint.map(|channel| {
            let base=f32::from(channel)*light;
            (base+(255.0-base)*glint).round() as u8
        });
        Color32::from_rgb(r,g,b)
    };
    mesh.colored_vertex(center,shade(0.0,0.0,1.0));
    for ring in 1..=RINGS {
        let theta=std::f32::consts::FRAC_PI_2*ring as f32/RINGS as f32;
        let (rho,z)=theta.sin_cos();
        for slice in 0..SLICES {
            let angle=std::f32::consts::TAU*slice as f32/SLICES as f32;
            let (y,x)=angle.sin_cos();
            let (x,y)=(rho*x,rho*y);
            mesh.colored_vertex(center+vec2(x,y)*radius,shade(x,y,z));
            let first=1+(ring-1)*SLICES;
            let at=first+slice;
            let next=first+(slice+1)%SLICES;
            if ring==1 { mesh.add_triangle(0,at,next); }
            else {
                mesh.add_triangle(at-SLICES,at,next);
                mesh.add_triangle(at-SLICES,next,next-SLICES);
            }
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}
/// Reuse the standard Settings row/checkbox while keeping the label clickable.
fn experimental_check_row(ui:&mut egui::Ui,width:f32,id:&'static str,label:&str,
    current:&mut bool,first:bool)->bool {
    let row=controls::settings_row(ui,width,label);
    let check=Rect::from_center_size(pos2(row.right()-12.0,row.center().y),vec2(24.0,24.0));
    let label_hit=Rect::from_min_max(row.min,pos2(check.left()-4.0,row.bottom()));
    let active=ui.is_enabled();
    let label_response=ui.interact(label_hit,ui.id().with((id,"label")),
        if active {Sense::CLICK} else {Sense::hover()});
    let mut next=*current;
    let check_response=controls::settings_check(ui,check,id,label,&mut next,active);
    if active && label_response.clicked() {
        next=!next;
        if first {check_response.request_focus();}
    }
    let changed=check_response.changed() || (active && label_response.clicked());
    if first && changed {*current=next;return true;}
    false
}
fn hit_axis(origin:V3,camera:math::Camera,size:[f64;2],pointer:[f64;2])->Option<usize> {
    let mut best=None;
    for axis in 0..3 {
        let Some([a,_,b])=camera.axis_projection(origin,axis,size) else {continue;};
        let d=[b[0]-a[0],b[1]-a[1]]; let n=d[0]*d[0]+d[1]*d[1];
        let t=(((pointer[0]-a[0])*d[0]+(pointer[1]-a[1])*d[1])/n).clamp(0.18,1.12);
        let dist=((pointer[0]-a[0]-t*d[0]).powi(2)+(pointer[1]-a[1]-t*d[1]).powi(2)).sqrt();
        if dist<10.0 && best.is_none_or(|(_,d)|dist<d) {best=Some((axis,dist));}
    } best.map(|(i,_)|i)
}
