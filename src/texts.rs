use phi_unpackd::cursor::Cursor;
use phi_unpackd::serialized;
use phi_unpackd::texture;
use phi_unpackd::unity;
use phi_unpackd::unityfs::Node;
use serde_json::{json, Map, Value};

pub const DIR: &str = "texts";
pub const INDEX: &str = "texts/index.json";
const DATA: &str = include_str!("../data/texts.json");
const LANGS: [&str; 5] = ["zh-CN", "zh-TW", "en", "ja", "ko"];

/// Where strings are looked for
#[derive(Default)]
pub struct Sources {
    /// `global-metadata.dat`
    pub metadata: Vec<u8>,
    /// The serialized files of `data.unity3d`
    pub scenes: Vec<Node>,
    /// `data.unity3d`'s streamed texture data for the shared assets an image is in
    pub streams: Vec<Node>,
    /// `classes*.dex`, `resources.arsc` and `res/` files
    pub apk: Vec<Vec<u8>>,
}

/// The shared assets files whose textures `data/texts.json` names
pub fn image_files() -> Vec<String> {
    let data: Value = serde_json::from_str(DATA).unwrap_or_default();
    let mut files: Vec<String> = groups(&data)
        .flat_map(|g| g["images"].as_array().cloned().unwrap_or_default())
        .filter_map(|i| i["file"].as_str().map(String::from))
        .collect();
    files.sort();
    files.dedup();
    files
}

fn groups(data: &Value) -> impl Iterator<Item = &Value> {
    data["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|s| s["groups"].as_array().into_iter().flatten())
}

pub struct Written {
    /// Output path and body
    pub files: Vec<(String, Vec<u8>)>,
    pub warnings: Vec<String>,
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn found(haystacks: &[&[u8]], text: &str) -> bool {
    let utf8 = text.as_bytes();
    haystacks
        .iter()
        .any(|h| memchr::memmem::find(h, utf8).is_some())
}

fn found_in_apk(apk: &[Vec<u8>], text: &str) -> bool {
    let wide = utf16(text);
    apk.iter().any(|h| {
        memchr::memmem::find(h, text.as_bytes()).is_some()
            || memchr::memmem::find(h, &wide).is_some()
    })
}

/// Keeps the languages of `line` the APK holds; None when it holds none of them
fn checked(line: &Value, check: &str, sources: &Sources, scene: &[&[u8]]) -> Option<Value> {
    let holds = |text: &str| match check {
        "metadata" => found(&[&sources.metadata], text),
        "scene" => found(scene, text),
        "apk" => found_in_apk(&sources.apk, text),
        _ => true,
    };
    let object = line.as_object()?;
    if let Some(text) = object.get("text").and_then(Value::as_str) {
        return holds(text).then(|| line.clone());
    }
    let kept: Map<String, Value> = LANGS
        .iter()
        .filter_map(|lang| {
            let text = object.get(*lang)?.as_str()?;
            holds(text).then(|| (lang.to_string(), json!(text)))
        })
        .collect();
    (!kept.is_empty()).then_some(Value::Object(kept))
}

/// The Texture2D called `name` in the serialized file `file`, as a PNG
fn image(sources: &Sources, file: &str, name: &str) -> Result<(Vec<u8>, u32, u32), String> {
    let node = sources
        .scenes
        .iter()
        .find(|n| n.name == file)
        .ok_or_else(|| format!("no {file}"))?;
    for object in serialized::parse(&node.bytes)?.objects {
        if object.class_id != unity::TEXTURE_2D {
            continue;
        }
        let Some(body) = object.bytes(&node.bytes) else {
            continue;
        };
        if Cursor::new(body).string().ok().as_deref() != Some(name) {
            continue;
        }
        let tex = texture::texture_object(body, &sources.streams)?;
        let png = texture::encode_png(&tex)?;
        return Ok((png, tex.width, tex.height));
    }
    Err(format!("{file} has no texture {name}"))
}

pub fn write(sources: &Sources) -> Result<Written, String> {
    let data: Value = serde_json::from_str(DATA).map_err(|e| e.to_string())?;
    let scene: Vec<&[u8]> = sources.scenes.iter().map(|n| n.bytes.as_slice()).collect();
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let mut index = Vec::new();
    let mut pv = Vec::new();
    for section in data["sections"].as_array().ok_or("no sections")? {
        let id = section["id"].as_str().ok_or("a section has no id")?;
        let mut groups = Vec::new();
        let mut lines_total = 0;
        for group in section["groups"].as_array().into_iter().flatten() {
            let gid = group["id"].as_str().unwrap_or_default();
            let check = group["check"].as_str().unwrap_or("none");
            let lines: Vec<Value> = group["lines"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|line| checked(line, check, sources, &scene))
                .collect();
            let dropped = group["lines"].as_array().map_or(0, Vec::len) - lines.len();
            if dropped > 0 {
                warnings.push(format!("{gid}: {dropped} lines are not in this APK"));
            }
            let mut images = Vec::new();
            for (n, wanted) in group["images"].as_array().into_iter().flatten().enumerate() {
                let (Some(file), Some(name)) = (wanted["file"].as_str(), wanted["name"].as_str())
                else {
                    continue;
                };
                match image(sources, file, name) {
                    Ok((png, width, height)) => {
                        let path = format!("{DIR}/images/{gid}-{n}.png");
                        images.push(json!({ "file": path, "width": width, "height": height }));
                        files.push((path, png));
                    }
                    Err(err) => warnings.push(format!("{gid}: {err}")),
                }
            }
            if lines.is_empty() && images.is_empty() {
                continue;
            }
            lines_total += lines.len();
            let mut out = json!({
                "id": gid,
                "title": group["title"],
                "note": group["note"],
                "lines": lines,
            });
            if !images.is_empty() {
                out["images"] = json!(images);
            }
            if let Some(clip) = group["pv"].as_str() {
                pv.push(json!({ "clip": clip, "section": id, "group": gid }));
            }
            groups.push(out);
        }
        index.push(json!({
            "id": id,
            "title": section["title"],
            "lede": section["lede"],
            "groups": groups.len(),
            "lines": lines_total,
        }));
        files.push((
            format!("{DIR}/{id}.json"),
            serde_json::to_vec(&json!({ "v": 1, "id": id, "groups": groups }))
                .map_err(|e| e.to_string())?,
        ));
    }
    files.push((
        INDEX.to_string(),
        serde_json::to_vec(&json!({ "v": 1, "sections": index, "pv": pv }))
            .map_err(|e| e.to_string())?,
    ));
    Ok(Written { files, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_what_the_apk_holds() {
        let sources = Sources {
            metadata: "“面对它，击溃它！”·“面對它，擊潰它！”".as_bytes().to_vec(),
            scenes: vec![Node {
                name: "level21".into(),
                bytes: b"\x05\0\0\0CONTENT".to_vec(),
            }],
            streams: vec![],
            apk: vec![utf16("账号登录")],
        };
        let scene: Vec<&[u8]> = sources.scenes.iter().map(|n| n.bytes.as_slice()).collect();
        let line =
            json!({ "zh-CN": "“面对它，击溃它！”", "zh-TW": "“面對它，擊潰它！”", "en": "gone" });
        assert_eq!(
            checked(&line, "metadata", &sources, &scene),
            Some(json!({ "zh-CN": "“面对它，击溃它！”", "zh-TW": "“面對它，擊潰它！”" }))
        );
        assert_eq!(
            checked(&json!({ "en": "gone" }), "metadata", &sources, &scene),
            None
        );
        assert!(checked(&json!({ "text": "CONTENT" }), "scene", &sources, &scene).is_some());
        assert!(checked(&json!({ "text": "账号登录" }), "apk", &sources, &scene).is_some());
        assert!(checked(&json!({ "text": "账号登录" }), "scene", &sources, &scene).is_none());
        assert!(checked(&json!({ "text": "anything" }), "none", &sources, &scene).is_some());
    }

    #[test]
    fn the_data_parses_and_names_its_images() {
        let data: Value = serde_json::from_str(DATA).unwrap();
        let ids: Vec<&str> = data["sections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["story", "chapter9", "ui", "data", "images", "sdk"]);
        assert!(groups(&data).count() > 200);
        assert!(image_files().contains(&"sharedassets30.assets".to_string()));
        let clips: Vec<&str> = groups(&data).filter_map(|g| g["pv"].as_str()).collect();
        assert_eq!(clips, ["qd", "after-deduction-video"]);
        let written = write(&Sources::default()).unwrap();
        let index: Value =
            serde_json::from_slice(&written.files.iter().find(|(p, _)| p == INDEX).unwrap().1)
                .unwrap();
        assert_eq!(index["sections"].as_array().unwrap().len(), 6);
    }
}
