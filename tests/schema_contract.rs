// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::{fs, path::Path, process::Command};

use serde_json::Value;

const SNAPSHOT: &str = "schemas/describe.json";

fn snapshot() -> Value {
    serde_json::from_str(include_str!("../schemas/describe.json")).unwrap()
}

#[test]
fn checked_in_contract_matches_describe() {
    let output = Command::new(env!("CARGO_BIN_EXE_kiecli"))
        .env_clear()
        .arg("describe")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let described: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        described == snapshot(),
        "{SNAPSHOT} is stale; run `cargo run --locked --quiet -- describe | jq -S . > {SNAPSHOT}`"
    );
    let schema = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|command| command["path"] == "describe")
        .unwrap();
    jsonschema::validate(&schema["stdout_schema"], &described).unwrap();
}

fn git(args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

type Version = [u64; 3];

fn version(raw: &str) -> Option<Version> {
    let mut parts = raw.split('.');
    let version = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(version)
}

fn allows_breaking_changes(old: Version, new: Version) -> bool {
    new[0] > old[0] || (old[0] == 0 && new[0] == 0 && new[1] > old[1])
}

fn latest_release<'a>(reachable: &'a str, published: Option<&str>) -> Option<(Version, &'a str)> {
    reachable
        .lines()
        .filter(|tag| {
            published.is_none_or(|published| published.lines().any(|released| released == *tag))
        })
        .filter_map(|tag| Some((version(tag.strip_prefix('v')?)?, tag)))
        .max()
}

#[test]
fn rejected_tags_cannot_become_the_compatibility_baseline() {
    let reachable = "v1.0.0\nv1.0.1\nv1.0.2\n";
    let published = "v1.0.0\nv1.1.0\n"; // 1.1.0 isn't reachable from this release.
    assert_eq!(
        latest_release(reachable, Some(published)),
        Some(([1, 0, 0], "v1.0.0"))
    );
}

#[test]
fn breaking_contract_changes_require_a_version_bump() {
    if !Path::new(env!("CARGO_MANIFEST_DIR")).join(".git").exists() {
        return; // Source packages have no release history.
    }
    let tags = git(&["tag", "--merged", "HEAD", "--list", "v[0-9]*"]);
    // CI knows which tags were published; an unpublished candidate is not a baseline.
    let published = std::env::var_os("KIECLI_TEST_PUBLISHED_TAGS")
        .map(|path| fs::read_to_string(path).expect("read the published release tag list"));
    let Some((released, tag)) = latest_release(&tags, published.as_deref()) else {
        return; // No release yet.
    };
    let current = version(env!("CARGO_PKG_VERSION")).unwrap();
    if allows_breaking_changes(released, current) {
        return;
    }
    let old: Value = serde_json::from_str(&git(&["show", &format!("{tag}:{SNAPSHOT}")])).unwrap();
    if let Some(part) = incompatible_part(&old, &snapshot()) {
        panic!(
            "Breaking change to {part} since {tag}; bump the major version (or the minor version before 1.0)"
        );
    }
}

fn incompatible_part(old: &Value, new: &Value) -> Option<String> {
    if !compatible_schema(&old["stderr_schema"], &new["stderr_schema"]) {
        return Some("stderr schema".to_owned());
    }
    for group in ["error_codes", "warning_codes"] {
        let new_codes = new[group].as_array().unwrap();
        for entry in old[group].as_array().unwrap() {
            if !new_codes
                .iter()
                .any(|candidate| candidate["code"] == entry["code"])
            {
                return Some(format!("{group}: removed {}", entry["code"]));
            }
        }
    }
    let commands = new["commands"].as_array().unwrap();
    for command in old["commands"].as_array().unwrap() {
        let path = command["path"].as_str().unwrap();
        let old_schema = &command["stdout_schema"];
        if !old_schema.is_null() {
            let new_schema = commands
                .iter()
                .find(|candidate| candidate["path"] == path)
                .map_or(&Value::Null, |candidate| &candidate["stdout_schema"]);
            if !compatible_schema(old_schema, new_schema) {
                return Some(format!("{path} stdout schema"));
            }
        }
    }
    None
}

fn compatible_schema(old: &Value, new: &Value) -> bool {
    retains_shape(old, new, old, new, &mut Vec::new())
}

// Compare the output shape, not general JSON Schema subsumption. Resolve local
// Schemars references so renaming a Rust type does not count as an API break.
fn retains_shape<'a>(
    old: &'a Value,
    new: &'a Value,
    old_root: &'a Value,
    new_root: &'a Value,
    visited: &mut Vec<(&'a Value, &'a Value)>,
) -> bool {
    let resolve = |node: &'a Value, root: &'a Value| {
        node["$ref"].as_str().map_or(node, |reference| {
            root.pointer(reference.strip_prefix('#').expect("local schema reference"))
                .expect("schema reference target")
        })
    };
    let (old, new) = (resolve(old, old_root), resolve(new, new_root));
    if visited
        .iter()
        .any(|&(o, n)| std::ptr::eq(o, old) && std::ptr::eq(n, new))
    {
        return true; // We already checked this recursive pair on the current path.
    }
    visited.push((old, new));
    let mut retains = |old, new| retains_shape(old, new, old_root, new_root, visited);
    let result = if !old.is_object() || !new.is_object() {
        old == new
    } else if let Some(variants) = alternatives(old) {
        variants.iter().all(|variant| {
            if let Some(candidates) = alternatives(new) {
                candidates
                    .iter()
                    .any(|candidate| retains(variant, candidate))
            } else {
                retains(variant, new)
            }
        })
    } else if let Some(candidates) = alternatives(new) {
        candidates.iter().any(|candidate| retains(old, candidate))
    } else {
        same_types(&old["type"], &new["type"])
            && enum_values(old)
                .iter()
                .all(|value| enum_values(new).contains(value))
            && contains_all(&old["required"], &new["required"])
            && old["dependentRequired"]
                .as_object()
                .into_iter()
                .flatten()
                .all(|(field, required)| contains_all(required, &new["dependentRequired"][field]))
            && old.get("if") == new.get("if")
            && old
                .get("then")
                .is_none_or(|then| retains(then, &new["then"]))
            && old["allOf"]
                .as_array()
                .into_iter()
                .flatten()
                .all(|old_clause| {
                    new["allOf"].as_array().is_some_and(|clauses| {
                        clauses
                            .iter()
                            .any(|new_clause| retains(old_clause, new_clause))
                    })
                })
            && old["properties"]
                .as_object()
                .into_iter()
                .flatten()
                .all(|(name, property)| {
                    new["properties"]
                        .get(name)
                        .is_some_and(|candidate| retains(property, candidate))
                })
            && ["items", "additionalProperties"]
                .into_iter()
                .all(|key| old.get(key).is_none_or(|child| retains(child, &new[key])))
            && old["prefixItems"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .all(|(index, child)| retains(child, &new["prefixItems"][index]))
    };
    visited.pop();
    result
}

fn contains_all(old: &Value, new: &Value) -> bool {
    old.as_array()
        .into_iter()
        .flatten()
        .all(|value| new.as_array().is_some_and(|values| values.contains(value)))
}

fn alternatives(schema: &Value) -> Option<&[Value]> {
    schema["anyOf"]
        .as_array()
        .or_else(|| schema["oneOf"].as_array())
        .map(Vec::as_slice)
}

fn enum_values(schema: &Value) -> &[Value] {
    if let Some(value) = schema.get("const") {
        std::slice::from_ref(value)
    } else {
        schema["enum"].as_array().map_or(&[], Vec::as_slice)
    }
}

fn same_types(old: &Value, new: &Value) -> bool {
    match (old.as_array(), new.as_array()) {
        (Some(old), Some(new)) => {
            old.len() == new.len() && old.iter().all(|kind| new.contains(kind))
        }
        _ => old == new,
    }
}

#[test]
fn contract_gate_distinguishes_additions_from_breaks() {
    use serde_json::json;

    let old = json!({
        "type": "object",
        "properties": {"rows": {"type": "array", "items": {"$ref": "#/$defs/Row"}}},
        "$defs": {"Row": {"type": "object", "properties": {
            "status": {"type": "string", "enum": ["partial", "complete"]},
            "amount": {"type": "string"}
        }}}
    });
    let mut new = old.clone();
    new["properties"]["rows"]["items"]["$ref"] = json!("#/$defs/RenamedRow");
    new["$defs"]["RenamedRow"] = new["$defs"]["Row"].take();
    let properties = new.pointer_mut("/$defs/RenamedRow/properties").unwrap();
    properties["reference"] = json!({"type": "string"});
    properties["status"]["enum"] = json!(["complete", "partial", "failed"]);
    assert!(compatible_schema(&old, &new));
    for (name, replacement) in [
        (
            "status",
            Some(json!({"type": "string", "const": "complete"})),
        ),
        ("amount", Some(json!({"type": "number"}))),
        ("amount", None),
    ] {
        let mut broken = new.clone();
        let properties = broken["$defs"]["RenamedRow"]["properties"]
            .as_object_mut()
            .unwrap();
        properties.remove(name);
        if let Some(replacement) = replacement {
            properties.insert(name.to_owned(), replacement);
        }
        assert!(
            !compatible_schema(&old, &broken),
            "accepted change to {name}"
        );
    }
    assert!(allows_breaking_changes([0, 1, 0], [0, 2, 0]));
    assert!(!allows_breaking_changes([0, 1, 0], [0, 1, 1]));
    assert!(!allows_breaking_changes([1, 1, 0], [1, 2, 0]));
    assert!(allows_breaking_changes([1, 1, 0], [2, 0, 0]));
}

#[test]
fn contract_gate_allows_new_diagnostic_codes_but_not_removals() {
    use serde_json::json;

    let old = snapshot();
    let mut new = old.clone();
    new["error_codes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"code": "example_new_error"}));
    assert_eq!(incompatible_part(&old, &new), None);
    for group in ["error_codes", "warning_codes"] {
        let mut broken = new.clone();
        let removed = broken[group].as_array_mut().unwrap().remove(0);
        assert_eq!(
            incompatible_part(&old, &broken),
            Some(format!("{group}: removed {}", removed["code"]))
        );
    }
}

#[test]
fn variant_specific_fields_must_stay_required() {
    use serde_json::json;

    let contract = snapshot();
    let old = &contract["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|command| command["path"] == "kontoabfrage offene-betraege")
        .unwrap()["stdout_schema"];
    let mut new = old.clone();
    let clauses = new["$defs"]["Teil"]["allOf"].as_array_mut().unwrap();
    clauses.reverse();
    assert!(compatible_schema(old, &new));

    let biene = new["$defs"]["Teil"]["allOf"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|clause| clause["if"]["properties"]["antwort_format"]["const"] == "biene")
        .unwrap();
    biene["then"]["required"] = json!(["zeilen"]);
    assert!(!compatible_schema(old, &new));
}
