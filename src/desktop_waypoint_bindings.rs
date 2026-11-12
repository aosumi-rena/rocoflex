//! Preserve waypoint key identity when route numbers change.
use serde_json::{Value, json};

fn supported(text: &str) -> Option<u8> {
    let bytes=text.as_bytes();
    (bytes.len()==1 && (bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit())).then(||bytes[0])
}
fn effective(row: &Value) -> Option<u8> {
    match row.get("keyBinding") {
        Some(Value::String(value))=>supported(value),
        Some(_)=>None,
        None=>supported(&row.get("id")?.as_u64()?.to_string()),
    }
}
pub(super) fn preserve_implicit(rows: &mut [Value]) -> Result<(),String> {
    for row in rows {
        let route=row.get("id").and_then(Value::as_u64).ok_or("Invalid waypoint route")?;
        let object=row.as_object_mut().ok_or("Invalid waypoint")?;
        if !object.contains_key("keyBinding") {
            let text=route.to_string();
            object.insert("keyBinding".into(),json!(if supported(&text).is_some(){text}else{String::new()}));
        }
    }
    Ok(())
}
pub(super) fn prepare_new(row:&mut Value, existing:&[Value], reserved:impl Fn(u8)->bool) -> Result<(),String> {
    let Some(key)=effective(row) else {return Ok(());};
    if reserved(key) || existing.iter().any(|other|effective(other)==Some(key)) {
        row.as_object_mut().ok_or("Invalid waypoint")?.insert("keyBinding".into(),json!(""));
    }
    Ok(())
}
