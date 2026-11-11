use crate::Result;
use serde::Serialize;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Serialize)]
pub struct Node {
    pub path: String,
    pub kind: String,
    pub tag: String,
    pub bytes: u32,
    pub children: u32,
}

#[derive(Serialize)]
pub struct ModelInfo {
    pub version: u32,
    pub nodes: Vec<Node>,
}

pub struct ModelNode<'a> {
    pub kind: u32,
    pub tag: u64,
    version: u32,
    pub data: &'a [u8],
    pub children: Vec<ModelNode<'a>>,
}

pub struct ModelTree<'a> {
    pub version: u32,
    pub root: ModelNode<'a>,
}

pub fn read_bytes(path: &Path, maximum_bytes: u64) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() { return Err("Select a parameter file.".into()); }
    if metadata.len() > maximum_bytes { return Err(format!("Parameter file exceeds the {maximum_bytes}-byte limit.")); }
    let bounded = maximum_bytes.checked_add(1).ok_or("Parameter byte limit exceeds the supported range")?;
    let mut bytes = Vec::new();
    file.take(bounded).read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    if bytes.len() as u64 > maximum_bytes { return Err(format!("Parameter file exceeds the {maximum_bytes}-byte limit.")); }
    Ok(bytes)
}

impl ModelNode<'_> {
    pub fn version(&self) -> u32 { self.version }

    pub(crate) fn require(&self, kind: u32, name: &[u8], tags: &[u64]) -> Result<()> {
        let accepted = if self.version == 1 {
            if name.len() > 8 {
                return Err("Invalid model operator name".into());
            }
            let mut tag = [0; 8];
            tag[..name.len()].copy_from_slice(name);
            self.tag == u64::from_le_bytes(tag)
        } else {
            tags.contains(&(self.tag ^ 0x40dab9846f846a19))
        };
        if self.kind != kind || !accepted {
            return Err("Unsupported model operator".into());
        }
        Ok(())
    }
}

impl<'a> ModelTree<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        fn word(bytes: &[u8], at: usize) -> Result<u32> {
            let end = at.checked_add(4).ok_or("Model offset overflow")?;
            Ok(u32::from_le_bytes(
                bytes
                    .get(at..end)
                    .ok_or("Truncated model")?
                    .try_into()
                    .unwrap(),
            ))
        }
        fn node<'a>(
            bytes: &'a [u8],
            at: &mut usize,
            version: u32,
            depth: usize,
            remaining: &mut usize,
        ) -> Result<ModelNode<'a>> {
            if depth > 64 || *remaining == 0 {
                return Err("Model tree exceeds limits".into());
            }
            *remaining -= 1;
            let kind = word(bytes, *at)?;
            if !matches!(kind, 0x7fca40ff | 0x7fca41ff) {
                return Err("Invalid model node".into());
            }
            let lo = word(bytes, *at + 4)?;
            let hi = word(bytes, *at + 8)?;
            let encoded = word(bytes, *at + 12)?;
            let count = if version == 2 {
                let value = encoded ^ lo ^ hi ^ 0xac5e7bd5;
                if !value.is_multiple_of(3) {
                    return Err("Invalid model child count".into());
                }
                value / 3
            } else {
                encoded
            } as usize;
            if count > *remaining {
                return Err("Model child count exceeds limits".into());
            }
            let size = word(bytes, *at + 16)? as usize;
            *at = at.checked_add(20).ok_or("Model offset overflow")?;
            let end = at.checked_add(size).ok_or("Model payload overflow")?;
            let data = bytes.get(*at..end).ok_or("Truncated model payload")?;
            *at = end;
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                children.push(node(bytes, at, version, depth + 1, remaining)?);
            }
            Ok(ModelNode {
                kind,
                tag: u64::from(lo) | (u64::from(hi) << 32),
                version,
                data,
                children,
            })
        }
        if word(bytes, 0)? != 0x7fca00ff {
            return Err("Invalid model header".into());
        }
        let version = word(bytes, 4)?;
        if !(1..=2).contains(&version) {
            return Err("Unsupported model version".into());
        }
        let mut at = 8;
        let root = node(bytes, &mut at, version, 0, &mut 100_000)?;
        if at != bytes.len() {
            return Err("Unexpected data after model tree".into());
        }
        Ok(Self { version, root })
    }
}

pub fn inspect(path: &Path) -> Result<ModelInfo> {
    let mut f = File::open(path).map_err(|e| format!("Cannot open model: {e}"))?;
    let length = f.metadata().map_err(|e| e.to_string())?.len();
    let mut header = [0; 8];
    f.read_exact(&mut header).map_err(|_| "Truncated model")?;
    if u32::from_le_bytes(header[..4].try_into().unwrap()) != 0x7fca00ff {
        return Err("Invalid model header".into());
    }
    let version = u32::from_le_bytes(header[4..].try_into().unwrap());
    if !(1..=2).contains(&version) {
        return Err(format!("Unsupported model format {version}"));
    }
    let mut nodes = Vec::new();
    fn read(
        f: &mut File,
        version: u32,
        length: u64,
        nodes: &mut Vec<Node>,
        path: String,
        depth: usize,
    ) -> Result<()> {
        if depth > 64 || nodes.len() >= 100_000 {
            return Err("Model tree exceeds limits".into());
        }
        let mut h = [0; 20];
        f.read_exact(&mut h).map_err(|_| "Truncated model node")?;
        let kind = match u32::from_le_bytes(h[..4].try_into().unwrap()) {
            0x7fca40ff => "component",
            0x7fca41ff => "module",
            _ => return Err("Invalid model node".into()),
        };
        let raw_tag = u64::from_le_bytes(h[4..12].try_into().unwrap());
        let mut children = u32::from_le_bytes(h[12..16].try_into().unwrap());
        if version == 2 {
            let packed = children ^ raw_tag as u32 ^ (raw_tag >> 32) as u32 ^ 0xac5e7bd5;
            if !packed.is_multiple_of(3) {
                return Err("Invalid model child count".into());
            }
            children = packed / 3;
        }
        if children > 100_000 {
            return Err("Model child count exceeds limits".into());
        }
        let bytes = u32::from_le_bytes(h[16..].try_into().unwrap());
        let end = f
            .stream_position()
            .map_err(|e| e.to_string())?
            .checked_add(u64::from(bytes))
            .ok_or("Model offset overflow")?;
        if end > length {
            return Err("Truncated model data".into());
        }
        let tag = if version == 1 {
            String::from_utf8_lossy(&h[4..12])
                .trim_end_matches('\0')
                .to_owned()
        } else {
            format!("{raw_tag:016x}")
        };
        f.seek(SeekFrom::Start(end)).map_err(|e| e.to_string())?;
        nodes.push(Node {
            path: path.clone(),
            kind: kind.into(),
            tag,
            bytes,
            children,
        });
        for i in 0..children {
            read(f, version, length, nodes, format!("{path}/{i}"), depth + 1)?;
        }
        Ok(())
    }
    read(&mut f, version, length, &mut nodes, "0".into(), 0)?;
    if f.stream_position().map_err(|e| e.to_string())? != length {
        return Err("Unexpected data after model tree".into());
    }
    Ok(ModelInfo { version, nodes })
}
