use crate::{Result, codec};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct Point {
    pub name: String,
    pub color: String,
    pub enabled: bool,
    pub data: Vec<f32>,
    pub reduced: Vec<f32>,
}

#[derive(Serialize)]
pub struct Source {
    pub name: String,
    pub color: String,
    pub enabled: bool,
    pub kind: String,
    pub data: Vec<f32>,
    pub points: Vec<Point>,
}

#[derive(Serialize)]
pub struct Preset {
    pub document: Value,
    pub name: String,
    pub probe: [f64; 3],
    pub transformation: [f32; 8],
    pub sources: Vec<Source>,
    pub basis_points: Vec<Vec<[f32; 2]>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Waypoint {
    pub route: u64,
    pub xy: [f64; 2],
    pub z: f64,
}

pub(crate) fn waypoints(document: &Value) -> Result<Vec<Waypoint>> {
    let voice = document.get("voice").unwrap_or(document);
    let rows: &[Value] = match voice.get("waypoints") {
        Some(value) => value.as_array().ok_or("Invalid waypoint list")?,
        None => &[],
    };
    if rows.len() > 256 { return Err("Too many waypoints".into()); }
    let depths = match document.get("rocoflexExperimentalCanvas").and_then(|v| v.get("waypointZ")) {
        Some(value) => Some(value.as_array().filter(|values| values.len() == rows.len())
            .ok_or("Invalid experimental waypoint depths")?),
        None => None,
    };
    let mut result: Vec<Waypoint> = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let route = row.get("id").and_then(Value::as_u64)
            .filter(|id| *id > 0).ok_or("Invalid waypoint route")?;
        if result.iter().any(|point| point.route == route) {
            return Err("Duplicate waypoint route".into());
        }
        row.get("midiBinding").and_then(Value::as_str).ok_or("Invalid waypoint binding")?;
        let mut xy = [0.0; 2];
        for (dst, key) in xy.iter_mut().zip(["x", "y"]) {
            *dst = row.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
                .ok_or("Invalid waypoint position")?;
        }
        let z = if let Some(depths) = depths {
            depths[index].as_f64().filter(|z| z.is_finite() && (-2.0..=2.0).contains(z))
                .ok_or("Invalid experimental waypoint depth")?
        } else { 0.0 };
        result.push(Waypoint { route, xy, z });
    }
    Ok(result)
}

fn name(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_owned()
}
fn enabled(v: &Value) -> bool {
    v["enabled"].as_bool().unwrap_or(true)
}
fn payload(v: &Value, key: &str, width: usize) -> Result<Vec<f32>> {
    codec::decode(
        v[key]
            .as_str()
            .ok_or_else(|| format!("Missing {key} payload"))?,
        width,
    )
}

#[derive(Clone, Copy)]
pub(crate) struct ExperimentalCanvasMetadata {
    pub enabled: bool,
    pub probe_z: f64,
    pub depth_axis: Option<[f64; 4]>,
    pub fixed_scale: Option<f64>,
    pub mover_enabled: bool,
}

/// Validate optional private experimental metadata without changing legacy
/// documents or any source/embedding payloads.
pub(crate) fn experimental_canvas_metadata(document: &Value) -> Result<Option<ExperimentalCanvasMetadata>> {
    let Some(value)=document.get("rocoflexExperimentalCanvas") else {return Ok(None);};
    if value.get("version").and_then(Value::as_u64)!=Some(1) {return Err("Unsupported experimental canvas version".into());}
    let enabled=value.get("enabled3d").and_then(Value::as_bool).ok_or("Invalid experimental canvas mode")?;
    let probe_z=value.get("probeZ").and_then(Value::as_f64)
        .filter(|z|z.is_finite() && (-2.0..=2.0).contains(z)).ok_or("Invalid experimental probe depth")?;
    let depth_axis=match value.get("depthAxis") {
        None | Some(Value::Null)=>None,
        Some(Value::Array(values)) if values.len()==4=>{
            let mut axis=[0.0;4];
            for (out,input) in axis.iter_mut().zip(values) {
                *out=input.as_f64().filter(|v|v.is_finite() && v.abs()<=1.000001)
                    .ok_or("Invalid experimental depth axis")?;
            }
            let norm:f64=axis.iter().map(|v|v*v).sum();
            if !(0.999..=1.001).contains(&norm) {return Err("Invalid experimental depth axis length".into());}
            Some(axis)
        }
        _=>return Err("Invalid experimental depth axis".into()),
    };
    let fixed_scale=match value.get("fixedScale") {
        None | Some(Value::Null)=>None,
        Some(scale)=>Some(scale.as_f64().filter(|n|n.is_finite() && (10.0..=500.0).contains(n))
            .ok_or("Invalid experimental canvas scale")?),
    };
    let mover_enabled=match value.get("probeMover") {
        None=>false,
        Some(on)=>on.as_bool().ok_or("Invalid experimental Probe Mover setting")?,
    };
    Ok(Some(ExperimentalCanvasMetadata {enabled,probe_z,depth_axis,fixed_scale,mover_enabled}))
}

impl Preset {
    pub fn parse(document: Value) -> Result<Self> {
        experimental_canvas_metadata(&document)?;
        waypoints(&document)?;
        let voice = document.get("voice").unwrap_or(&document);
        let array = voice["sources"]
            .as_array()
            .ok_or("Missing preset sources")?;
        if array.len() > 256 {
            return Err("Too many sources".into());
        }
        let mut sources = Vec::new();
        for (i, source) in array.iter().enumerate() {
            let parse_source = || -> Result<Source> {
                let mut points = Vec::new();
                let kind = name(source, "type");
                if kind == "point" {
                    points.push(Point {
                        name: name(source, "name"),
                        color: name(source, "color"),
                        enabled: enabled(source),
                        data: payload(source, "data", 128)?,
                        reduced: payload(source, "reducedData", 4)?,
                    });
                } else if kind == "trajectory" {
                    let rows = source.get("points").ok_or("Missing trajectory points")?;
                    let rows = rows.as_array().ok_or("Invalid point list")?;
                    if rows.len() > 4096 {
                        return Err("Too many points".into());
                    }
                    for (j, row) in rows.iter().enumerate() {
                        points.push(Point {
                            name: name(row, "name"),
                            color: name(row, "color"),
                            enabled: enabled(row),
                            data: payload(row, "data", 128)
                                .map_err(|e| format!("Point {j}: {e}"))?,
                            reduced: payload(row, "reducedData", 4)
                                .map_err(|e| format!("Point {j}: {e}"))?,
                        });
                    }
                } else {
                    return Err(format!("Unsupported source type: {kind}"));
                }
                Ok(Source {
                    name: name(source, "name"),
                    color: name(source, "color"),
                    enabled: enabled(source),
                    kind,
                    data: payload(source, "data", 128)?,
                    points,
                })
            };
            sources.push(parse_source().map_err(|e| format!("Source {i}: {e}"))?);
        }
        if sources.iter().map(|s| s.points.len()).sum::<usize>() > 32768 {
            return Err("Too many total points".into());
        }
        let mut transformation = [0.0; 8];
        if let Some(v) = voice.get("transformation") {
            let v = v
                .as_array()
                .filter(|a| a.len() == 8)
                .ok_or("Expected eight transformation values")?;
            for (dst, n) in transformation.iter_mut().zip(v) {
                *dst = n.as_f64().ok_or("Invalid transformation")? as f32;
                if !dst.is_finite() {
                    return Err("Invalid transformation".into());
                }
            }
        } else {
            return Err("Missing preset transformation".into());
        }
        let mut probe = [0.0, 0.0, 0.1];
        for (i, key) in ["probeLocationX", "probeLocationY", "probeRadius"]
            .iter()
            .enumerate()
        {
            if let Some(v) = voice.get(key) {
                probe[i] = v
                    .as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or("Invalid probe")?;
            }
        }
        if probe[2] < 0.0 {
            return Err("Invalid probe radius".into());
        }
        let basis_points: Vec<Vec<[f32; 2]>> = sources
            .iter()
            .map(|s| {
                s.points
                    .iter()
                    .map(|p| {
                        let mut xy = [0.0; 2];
                        for row in 0..2 {
                            for col in 0..4 {
                                xy[row] += p.reduced[col] * transformation[row * 4 + col];
                            }
                        }
                        xy
                    })
                    .collect()
            })
            .collect();
        if basis_points
            .iter()
            .flatten()
            .flatten()
            .any(|n| !n.is_finite())
        {
            return Err("Projection exceeds float range".into());
        }
        Ok(Self {
            name: name(&document, "presetName"),
            document,
            probe,
            transformation,
            sources,
            basis_points,
        })
    }
}
