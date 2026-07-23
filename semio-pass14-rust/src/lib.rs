use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn obj(v: &Value) -> Result<&Map<String, Value>, String> {
    v.as_object().ok_or_else(|| "expected object".into())
}
fn arr(v: &Value) -> Result<&Vec<Value>, String> {
    v.as_array().ok_or_else(|| "expected array".into())
}
fn string(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "expected string".into())
}
fn take(map: &mut Map<String, Value>, key: &str) -> Result<Value, String> {
    map.remove(key).ok_or_else(|| format!("missing {key}"))
}

pub fn form_to_item(v: &Value) -> Result<Value, String> {
    let m = obj(v)?;
    if m.get("tag").and_then(Value::as_str) != Some("Ref") {
        return Err("expression form must be Ref".into());
    }
    let kind = string(m.get("ref_kind").ok_or("missing ref_kind")?)?;
    if !matches!(kind, "Node" | "Expression" | "Quote" | "Variable" | "Scope") {
        return Err("bad ref kind".into());
    }
    Ok(serde_json::json!({"kind": kind, "id": string(m.get("id").ok_or("missing id")?)?}))
}

pub fn form_to_payload(v: &Value) -> Result<Value, String> {
    if v.is_null() {
        return Ok(Value::Null);
    }
    let m = obj(v)?;
    match m.get("tag").and_then(Value::as_str) {
        Some("Ref") => {
            let kind = string(m.get("ref_kind").ok_or("missing ref_kind")?)?;
            let id = string(m.get("id").ok_or("missing id")?)?;
            if matches!(kind, "Expression" | "Scope") {
                Ok(serde_json::json!({"kind": kind, "id": id}))
            } else if matches!(kind, "Node" | "Quote" | "Variable") {
                Ok(serde_json::json!({"kind": "Item", "item": {"kind": kind, "id": id}}))
            } else {
                Err("bad ref kind".into())
            }
        }
        Some("Apply") => {
            let op = string(m.get("operator").ok_or("missing operator")?)?;
            let args = arr(m.get("args").ok_or("missing args")?)?;
            match op {
                "Identity" | "Direction" if args.len() == 2 => Ok(serde_json::json!({
                    "kind": op,
                    "lhs": form_to_payload(&args[0])?,
                    "rhs": form_to_payload(&args[1])?
                })),
                "Constraint" | "Query" if args.len() == 1 => Ok(serde_json::json!({
                    "kind": op,
                    "target": form_to_payload(&args[0])?
                })),
                "Activation" if args.len() == 1 => Ok(serde_json::json!({
                    "kind": op,
                    "requested_context": Value::Null,
                    "target": form_to_payload(&args[0])?
                })),
                "Activation" if args.len() == 2 => Ok(serde_json::json!({
                    "kind": op,
                    "requested_context": form_to_payload(&args[0])?,
                    "target": form_to_payload(&args[1])?
                })),
                _ => Err(format!("invalid operator/arity {op}/{}", args.len())),
            }
        }
        _ => Err("bad form tag".into()),
    }
}

fn decode_trace(mut trace: Value) -> Result<Value, String> {
    let m = trace.as_object_mut().ok_or("trace object")?;
    for key in ["activation_target", "requested_context"] {
        if let Some(v) = m.get_mut(key) {
            if !v.is_null() {
                *v = form_to_payload(v)?;
            }
        }
    }
    if let Some(v) = m.get_mut("rhs_forms_instantiated") {
        for x in v.as_array_mut().ok_or("rhs array")? {
            *x = form_to_payload(x)?;
        }
    }
    if let Some(v) = m.get_mut("rule_discovery") {
        for x in v.as_array_mut().ok_or("rule discovery array")? {
            let q = x.as_object_mut().ok_or("rule discovery object")?;
            if let Some(r) = q.get_mut("request") {
                *r = form_to_payload(r)?;
            }
            if let Some(r) = q.get_mut("procedure_reference") {
                if !r.is_null() {
                    *r = form_to_payload(r)?;
                }
            }
        }
    }
    if let Some(v) = m.get_mut("capability_exposures_considered") {
        for x in v.as_array_mut().ok_or("capability array")? {
            let q = x.as_object_mut().ok_or("capability object")?;
            if let Some(r) = q.get_mut("selector") {
                *r = form_to_payload(r)?;
            }
        }
    }
    if let Some(v) = m.get_mut("blockable_event_proposals") {
        for x in v.as_array_mut().ok_or("proposal array")? {
            let q = x.as_object_mut().ok_or("proposal object")?;
            if let Some(r) = q.get_mut("form") {
                *r = form_to_payload(r)?;
            }
        }
    }
    Ok(trace)
}

fn decode_payload(mut payload: Value) -> Result<(Value, Option<(u64, Value)>), String> {
    let m = payload.as_object_mut().ok_or("payload object")?;
    let kind = string(m.get("kind").ok_or("payload kind")?)?.to_string();
    let mut evidence = None;
    match kind.as_str() {
        "Identity" | "Direction" => {
            for key in ["lhs", "rhs"] {
                let v = m.get_mut(key).ok_or("binary payload field")?;
                *v = form_to_payload(v)?;
            }
        }
        "Constraint" | "Query" => {
            let v = m.get_mut("target").ok_or("target")?;
            *v = form_to_payload(v)?;
        }
        "ActivationIntent" => {
            if let Some(v) = m.get_mut("requested_context") {
                if !v.is_null() {
                    *v = form_to_payload(v)?;
                }
            }
            let v = m.get_mut("target").ok_or("target")?;
            *v = form_to_payload(v)?;
        }
        "ActivationEvent" => {
            if let Some(v) = m.get_mut("requested_context") {
                if !v.is_null() {
                    *v = form_to_payload(v)?;
                }
            }
            let v = m.get_mut("target").ok_or("target")?;
            *v = form_to_payload(v)?;
            let projection = take(m, "evidence_projection")?;
            let p = obj(&projection)?;
            let order = p.get("trace_order").and_then(Value::as_u64).ok_or("trace_order")?;
            let trace = decode_trace(p.get("trace").ok_or("trace")?.clone())?;
            let result = m.get_mut("result").and_then(Value::as_object_mut).ok_or("result")?;
            let id = take(result, "trace_projection_id")?;
            result.insert("trace".into(), id);
            evidence = Some((order, trace));
        }
        "Presence" => {
            if m.len() != 2 || !m.contains_key("expression") {
                return Err("explicit Presence shape".into());
            }
        }
        _ => {}
    }
    Ok((payload, evidence))
}

pub fn reload_candidate(candidate: &Value) -> Result<Value, String> {
    let c = obj(candidate)?;
    if c.contains_key("traces") || c.contains_key("entities") {
        return Err("forbidden top-level authority or held mutation".into());
    }
    if c.get("schema_version").and_then(Value::as_str) != Some("semio-reduction-candidate-2.0") {
        return Err("candidate version".into());
    }
    for key in ["nodes", "quotes", "variables"] {
        if !c.get(key).map(Value::is_array).unwrap_or(false) {
            return Err(format!("missing canonical entity category {key}"));
        }
    }
    let mut out = Map::new();
    for (key, value) in c {
        if !matches!(key.as_str(), "schema_version" | "canonical_schema_version" | "expressions" | "records") {
            out.insert(key.clone(), value.clone());
        }
    }
    out.insert("schema_version".into(), c.get("canonical_schema_version").ok_or("canonical version")?.clone());
    let mut expressions = Vec::new();
    for expression in arr(c.get("expressions").ok_or("expressions")?)? {
        let em = obj(expression)?;
        let mut items = Vec::new();
        for form in arr(em.get("forms").ok_or("forms")?)? {
            items.push(form_to_item(form)?);
        }
        expressions.push(serde_json::json!({
            "id": em.get("id").ok_or("expr id")?,
            "items": items,
            "provenance": em.get("provenance").ok_or("expr provenance")?
        }));
    }
    out.insert("expressions".into(), Value::Array(expressions));
    let mut records = Vec::new();
    let mut traces: Vec<(u64, Value)> = Vec::new();
    for record in arr(c.get("records").ok_or("records")?)? {
        let mut rm = obj(record)?.clone();
        let payload = take(&mut rm, "payload")?;
        let (decoded, evidence) = decode_payload(payload)?;
        rm.insert("payload".into(), decoded);
        records.push(Value::Object(rm));
        if let Some(value) = evidence {
            traces.push(value);
        }
    }
    traces.sort_by_key(|x| x.0);
    out.insert("records".into(), Value::Array(records));
    out.insert("traces".into(), Value::Array(traces.into_iter().map(|x| x.1).collect()));
    Ok(Value::Object(out))
}

fn canonical_write(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => out.push_str(&serde_json::to_string(s).unwrap()),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 { out.push(','); }
                canonical_write(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            let mut ordered: BTreeMap<&str, &Value> = BTreeMap::new();
            for (k, v) in m { ordered.insert(k, v); }
            for (i, (k, v)) in ordered.into_iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push_str(&serde_json::to_string(k).unwrap());
                out.push(':');
                canonical_write(v, out);
            }
            out.push('}');
        }
    }
}

pub fn canonical_json(v: &Value) -> String {
    let mut s = String::new();
    canonical_write(v, &mut s);
    s
}

pub fn digest_value(v: &Value) -> String {
    let mut h = Sha256::new();
    h.update(canonical_json(v).as_bytes());
    format!("{:x}", h.finalize())
}

pub fn load_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
