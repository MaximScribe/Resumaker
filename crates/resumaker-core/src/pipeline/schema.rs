//! JSON Schemas the model's answers must follow.

use serde_json::{Map, Value, json};

fn string() -> Value {
    json!({ "type": "string" })
}

fn strings() -> Value {
    json!({ "type": "array", "items": { "type": "string" } })
}

fn object(fields: &[(&str, Value)]) -> Value {
    let props: Map<String, Value> = fields
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();
    json!({ "type": "object", "properties": props })
}

/// An entry type with a `source` id field.
fn sourced(fields: &[&str], lists: &[&str]) -> Value {
    let mut all: Vec<(&str, Value)> = fields.iter().map(|f| (*f, string())).collect();
    all.extend(lists.iter().map(|l| (*l, strings())));
    all.push((
        "source",
        json!({ "type": "string", "description": "id of the source this entry comes from" }),
    ));
    json!({ "type": "array", "items": object(&all) })
}

/// The profile (JSON Resume with `source` ids), without the `sources` list.
pub fn profile() -> Value {
    let location = object(&[
        ("city", string()),
        ("region", string()),
        ("countryCode", string()),
    ]);
    let profiles = json!({
        "type": "array",
        "items": object(&[("network", string()), ("username", string()), ("url", string())]),
    });
    object(&[
        (
            "basics",
            object(&[
                ("name", string()),
                ("label", string()),
                ("email", string()),
                ("phone", string()),
                ("url", string()),
                ("summary", string()),
                ("location", location),
                ("profiles", profiles),
            ]),
        ),
        (
            "work",
            sourced(
                &[
                    "name",
                    "position",
                    "location",
                    "url",
                    "startDate",
                    "endDate",
                    "summary",
                ],
                &["highlights"],
            ),
        ),
        (
            "volunteer",
            sourced(
                &[
                    "organization",
                    "position",
                    "startDate",
                    "endDate",
                    "summary",
                ],
                &["highlights"],
            ),
        ),
        (
            "education",
            sourced(
                &[
                    "institution",
                    "area",
                    "studyType",
                    "startDate",
                    "endDate",
                    "score",
                ],
                &["courses"],
            ),
        ),
        (
            "awards",
            sourced(&["title", "date", "awarder", "summary"], &[]),
        ),
        (
            "certificates",
            sourced(&["name", "date", "issuer", "url"], &[]),
        ),
        (
            "publications",
            sourced(&["name", "publisher", "releaseDate", "url", "summary"], &[]),
        ),
        ("skills", sourced(&["name", "level"], &["keywords"])),
        ("languages", sourced(&["language", "fluency"], &[])),
        ("interests", sourced(&["name"], &["keywords"])),
        (
            "projects",
            sourced(
                &["name", "description", "startDate", "endDate", "url"],
                &["highlights", "keywords"],
            ),
        ),
    ])
}

/// The tailoring answer: who the resume is for, plus the tailored profile.
pub fn tailored() -> Value {
    let mut schema = object(&[
        (
            "target",
            object(&[("company", string()), ("role", string())]),
        ),
        ("resume", profile()),
    ]);
    schema["required"] = json!(["resume"]);
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_are_objects_with_properties() {
        for s in [profile(), tailored()] {
            assert_eq!(s["type"], "object");
            assert!(s["properties"].is_object());
        }
        assert_eq!(
            profile()["properties"]["work"]["items"]["properties"]["source"]["type"],
            "string"
        );
    }
}
