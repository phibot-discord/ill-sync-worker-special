//! Reads everything this tool owns from an APK into an output tree laid out like the R2 bucket, and lists it for
//! publishing: the hidden song (`c9s6/`), the passcode entry's art and tile (`collection-art/blackhole0*`) and
//! `collection-art/specials.json`, which the web UI lays over phi-unpackd's `collection-art/index.json` by `id`

use crate::art;
use crate::before;
use crate::c9s::{self, Part};
use crate::cutscene;
use crate::deduction;
use crate::texts;
use crate::tile;
use phi_unpackd::audio;
use phi_unpackd::catalog;
use phi_unpackd::collection::{self, Text};
use phi_unpackd::collection_art;
use phi_unpackd::extract;
use phi_unpackd::files::write_if_changed;
use phi_unpackd::pez;
use phi_unpackd::publish::Upload;
use phi_unpackd::pv;
use phi_unpackd::texture;
use phi_unpackd::unityfs;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

pub const OVERLAY: &str = "collection-art/specials.json";
/// Where the APK keeps Unity's player data; a scene's `.resource` streams sit loose here, outside data.unity3d
const DATA_DIR: &str = "assets/bin/Data/";

/// What `extract` wrote and what it could not read
pub struct Report {
    pub files: Vec<String>,
    pub warnings: Vec<String>,
}

/// The passcode entry as the map places it: its collection key and sub, and the map scene its tile is on
struct Placement {
    key: String,
    sub: i32,
    scene: String,
}

/// What this tool reads from an APK: the `c9s.*` bundles it uses (decrypted), the il2cpp strings, level0 and every
/// scene by its build path
pub struct Inputs {
    pub art: Vec<u8>,
    pub tile: Vec<u8>,
    pub chart: Vec<u8>,
    pub music: Vec<u8>,
    pub background: Vec<u8>,
    pub cutscene: Vec<u8>,
    pub metadata: Vec<u8>,
    pub level0: Vec<u8>,
    pub scenes: Vec<(String, Vec<u8>)>,
    /// The Deduction scene with its shared assets, their streamed data and globalgamemanagers.assets
    pub deduction: Vec<unityfs::Node>,
    /// Where the game texts are looked for
    pub texts: texts::Sources,
}

pub fn extract(apk: &Path, out: &Path) -> Result<Report, String> {
    let inputs = read_apk(apk)?;
    let mut report = write_all(&inputs, out)?;
    let written = texts::write(&inputs.texts)?;
    report.warnings.extend(written.warnings);
    for (rel, body) in written.files {
        put(out, &mut report.files, &rel, &body)?;
    }
    Ok(report)
}

pub fn read_apk(apk: &Path) -> Result<Inputs, String> {
    let (mut zip, names) = extract::open_apk(apk)?;
    let catalog = extract::read_catalog(&mut zip, &names)?;
    let found: HashMap<String, String> = catalog::locate(&catalog, |key| {
        c9s::is_encrypted(key).then(|| key.to_string())
    })?
    .into_iter()
    .collect();
    let mut open = |key: &str| -> Result<Vec<u8>, String> {
        let path = found
            .get(key)
            .ok_or_else(|| format!("{key} is not in this apk"))?;
        c9s::decrypt(&extract::read_zip(&mut zip, path)?).map_err(|e| format!("{key}: {e}"))
    };
    let art = open(art::KEY)?;
    let tile = open(c9s::TILE)?;
    let chart = open(c9s::part_key(Part::Chart))?;
    let music = open(c9s::part_key(Part::Music))?;
    let background = open(c9s::part_key(Part::Background))?;
    let cutscene = open(c9s::CUTSCENE)?;
    let metadata = extract::read_zip(&mut zip, extract::METADATA)?;
    let packed = extract::read_packed(&mut zip, &names)?.ok_or("apk has no data.unity3d")?;
    let mut nodes: HashMap<String, Vec<u8>> =
        unityfs::decode_where(&packed, |n| !unityfs::is_resource(n))?
            .into_iter()
            .map(|n| (n.name, n.bytes))
            .collect();
    let level0 = nodes.get("level0").ok_or("no level0")?.clone();
    let wanted = texts::image_files();
    let streams = unityfs::decode_where(&packed, |name| {
        name.ends_with(".resS") && wanted.iter().any(|file| name.starts_with(file.as_str()))
    })?;
    let mut text_scenes: Vec<unityfs::Node> = nodes
        .iter()
        .map(|(name, bytes)| unityfs::Node {
            name: name.clone(),
            bytes: bytes.clone(),
        })
        .collect();
    text_scenes.sort_by(|a, b| a.name.cmp(&b.name));
    let mut apk_files = Vec::new();
    for name in names.iter().filter(|n| {
        (n.starts_with("classes") && n.ends_with(".dex"))
            || n.as_str() == "resources.arsc"
            || (n.starts_with("res/") && n.ends_with(".xml"))
    }) {
        apk_files.push(extract::read_zip(&mut zip, name)?);
    }
    let mut deduction = scene_nodes(&packed, &nodes, deduction::SCENE)?;
    let own = deduction
        .iter()
        .find(|n| n.name.starts_with("sharedassets") && n.name.ends_with(".assets"))
        .map(|n| n.name.trim_end_matches(".assets").to_string());
    if let Some(own) = own {
        let loose = format!("{DATA_DIR}{own}.resource");
        if names.contains(&loose) {
            deduction.push(unityfs::Node {
                name: format!("{own}.resource"),
                bytes: extract::read_zip(&mut zip, &loose)?,
            });
        }
    }
    let scenes = pv::scenes_in(&nodes)?
        .into_iter()
        .enumerate()
        .filter_map(|(n, scene)| Some((scene, nodes.remove(&format!("level{n}"))?)))
        .collect();
    Ok(Inputs {
        art,
        tile,
        chart,
        music,
        background,
        cutscene,
        texts: texts::Sources {
            metadata: metadata.clone(),
            scenes: text_scenes,
            streams,
            apk: apk_files,
        },
        metadata,
        level0,
        scenes,
        deduction,
    })
}

/// A scene by its build path, with the files its objects reference: its level, its shared assets and their streamed
/// data, and globalgamemanagers.assets
fn scene_nodes(
    packed: &[u8],
    nodes: &HashMap<String, Vec<u8>>,
    scene: &str,
) -> Result<Vec<unityfs::Node>, String> {
    let n = pv::scenes_in(nodes)?
        .iter()
        .position(|s| s == scene)
        .ok_or_else(|| format!("no scene {scene}"))?;
    let own = format!("sharedassets{n}");
    let mut found: Vec<unityfs::Node> = [
        format!("level{n}"),
        format!("{own}.assets"),
        "globalgamemanagers.assets".into(),
    ]
    .into_iter()
    .filter_map(|name| {
        let bytes = nodes.get(&name)?.clone();
        Some(unityfs::Node { name, bytes })
    })
    .collect();
    found.extend(unityfs::decode_where(packed, |name| {
        unityfs::is_resource(name) && name.starts_with(&format!("{own}."))
    })?);
    Ok(found)
}

/// Writes every file this tool owns into `out`, laid out like the bucket
pub fn write_all(inputs: &Inputs, out: &Path) -> Result<Report, String> {
    let mut warnings = Vec::new();
    let art = art::read(&inputs.art).map_err(|e| format!("{}: {e}", art::KEY))?;
    let (tile, tex) = tile::read_tile(&unityfs::decode(&inputs.tile)?)?;
    let tile_png = texture::encode_png(&tex)?;
    if tile.id != art::ID {
        return Err(format!(
            "the passcode tile opens {}, not {}",
            tile.id,
            art::ID
        ));
    }
    let (chart, (notes, _)) = extract::carve_chart(&unityfs::decode(&inputs.chart)?)?;
    let music = audio::ogg_from_nodes(&unityfs::decode(&inputs.music)?)?;
    let background = texture::texture_from_bundle(&unityfs::decode(&inputs.background)?)?;
    let (width, height) = (background.width, background.height);
    let background = texture::encode_png(&background)?;
    let (lines, bpm) =
        c9s::chart_shape(&chart).map_err(|e| format!("{}: {e}", Part::Chart.path()))?;
    let cut = cutscene::read(&unityfs::decode(&inputs.cutscene)?, &mut warnings)?;
    let (texts, missing) =
        before::before_texts(|text| extract::find(&inputs.metadata, text).is_some());
    warnings.extend(missing);
    let placement = placement(&inputs.level0, &inputs.scenes)?;
    let deduction = if inputs.deduction.is_empty() {
        None
    } else {
        Some(deduction::read(&inputs.deduction, &mut warnings)?)
    };

    let mut files = Vec::new();
    let mut write = |rel: &str, body: &[u8]| put(out, &mut files, rel, body);
    write(&collection_art::png_file(art::ID), &art.png)?;
    write(&collection_art::jpg_file(art::ID), &art.jpg)?;
    write(&tile::tile_file(&tile.id), &tile_png)?;
    write(Part::Chart.path(), &chart)?;
    write(Part::Music.path(), &music)?;
    write(Part::Background.path(), &background)?;
    for (rel, body) in cut.files() {
        write(&rel, body)?;
    }
    let shown = cut.json(std::slice::from_ref(&art.sprites), &mut warnings);
    let index = c9s::index(
        &c9s::Song {
            lines,
            bpm,
            notes,
            width,
            height,
        },
        shown,
    )?;
    write(c9s::INDEX, &index)?;
    write(OVERLAY, &overlay(&placement, &art.sprites, &tile, &texts)?)?;
    if let Some(deduction) = &deduction {
        for (rel, body) in deduction.files() {
            write(&rel, body)?;
        }
        write(deduction::INDEX, &deduction.index()?)?;
    }
    pez::pack_one(
        &out.join(c9s::PACK),
        c9s::ID,
        &c9s::pack_info(),
        &chart,
        &background,
        &music,
    )?;
    files.push(c9s::PACK.to_string());
    Ok(Report { files, warnings })
}

fn put(out: &Path, files: &mut Vec<String>, rel: &str, body: &[u8]) -> Result<(), String> {
    let path = out.join(rel);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    write_if_changed(&path, body)?;
    files.push(rel.to_string());
    Ok(())
}

/// The passcode entry's key and sub from the Collection, and the map scene whose locked tile asks for the passcode
fn placement(level0: &[u8], scenes: &[(String, Vec<u8>)]) -> Result<Placement, String> {
    let items = collection::parse_items(level0)?;
    let (key, sub) = match collection_art::entry_ids(&items).remove(art::ID).as_deref() {
        Some([one]) => one.clone(),
        other => return Err(format!("{} is the collection id of {other:?}", art::ID)),
    };
    let scene = scenes
        .iter()
        .find(|(_, level)| art::has_passcode_call(level))
        .map(|(scene, _)| scene.clone())
        .ok_or("no map has the passcode button")?;
    Ok(Placement { key, sub, scene })
}

/// `collection-art/specials.json`: the fields phi-unpackd's manifest leaves to this tool, by entry id
fn overlay(
    placed: &Placement,
    art: &art::Sprites,
    tile: &tile::Tile,
    before: &BTreeMap<String, Text>,
) -> Result<Vec<u8>, String> {
    let mut entries = vec![json!({
        "id": art::ID,
        "key": placed.key,
        "sub": placed.sub,
        "layout": art::LAYOUT,
        "scene": placed.scene,
        "art": {
            "file": collection_art::jpg_file(art::ID),
            "original": collection_art::png_file(art::ID),
            "source": art::KEY,
            "width": art.width,
            "height": art.height,
        },
        "tile": {
            "file": tile::tile_file(&tile.id),
            "width": tile.width,
            "height": tile.height,
            "label": tile.label,
        },
    })];
    for (id, text) in before {
        entries.push(json!({
            "id": id,
            "before": { "flag": before::FLAG, "text": collection::by_lang(text) },
        }));
    }
    serde_json::to_vec(&json!({ "v": 1, "entries": entries })).map_err(|e| e.to_string())
}

/// What `publish` sends from an output tree `extract` wrote: the files, then the two manifests that name them
pub fn uploads(out: &Path) -> Result<(Vec<Upload>, Vec<Upload>), String> {
    let mut files = Vec::new();
    let mut manifests = Vec::new();
    let mut add = |rel: String| -> Result<(), String> {
        let path = out.join(&rel);
        if !path.is_file() {
            return Err(format!("{rel} is missing; run extract first"));
        }
        let upload = Upload {
            path,
            mime: mime(&rel)?,
            key: rel,
        };
        if upload.key == c9s::INDEX
            || upload.key == OVERLAY
            || upload.key == deduction::INDEX
            || (upload.key.starts_with(texts::DIR) && upload.key.ends_with(".json"))
        {
            manifests.push(upload);
        } else {
            files.push(upload);
        }
        Ok(())
    };
    let mut song: Vec<String> = fs::read_dir(out.join(c9s::DIR))
        .map_err(|e| format!("{}: {e}", c9s::DIR))?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .map(|name| format!("{}/{name}", c9s::DIR))
        .collect();
    song.sort();
    for rel in song {
        add(rel)?;
    }
    for rel in [
        collection_art::png_file(art::ID),
        collection_art::jpg_file(art::ID),
        tile::tile_file(art::ID),
        OVERLAY.to_string(),
    ] {
        add(rel)?;
    }
    let mut deduction_files = Vec::new();
    collect(
        &out.join(deduction::DIR),
        deduction::DIR,
        &mut deduction_files,
    )?;
    deduction_files.sort();
    for rel in deduction_files {
        add(rel)?;
    }
    let mut text_files = Vec::new();
    collect(&out.join(texts::DIR), texts::DIR, &mut text_files)?;
    text_files.sort();
    for rel in text_files {
        add(rel)?;
    }
    manifests.sort_by(|a, b| (a.key == texts::INDEX, &a.key).cmp(&(b.key == texts::INDEX, &b.key)));
    Ok((files, manifests))
}

/// Every file under `dir`, as keys under `prefix`; nothing when `dir` does not exist
fn collect(dir: &Path, prefix: &str, out: &mut Vec<String>) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let key = format!("{prefix}/{name}");
        if entry.path().is_dir() {
            collect(&entry.path(), &key, out)?;
        } else {
            out.push(key);
        }
    }
    Ok(())
}

fn mime(key: &str) -> Result<&'static str, String> {
    Ok(match key.rsplit_once('.').map(|(_, ext)| ext) {
        Some("json") => "application/json",
        Some("ogg") => "audio/ogg",
        Some("mp4") => "video/mp4",
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("pez") => "application/octet-stream",
        _ => return Err(format!("{key}: no content type for it")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixture;
    use serde_json::Value;
    use sha2::{Digest, Sha256};

    /// Every object this tool owns, as R2 holds it after the 2026-10-17 publish
    const LIVE: [(&str, &str); 12] = [
        (
            "c9s6/C9S6-SP.pez",
            "6bde03cb31f31d14829942aa743a0eba20f7590e02ff04af0197220deb74ffda",
        ),
        (
            "c9s6/background.png",
            "deeef437a19d2b4c9029893dac6ffa2ae49753b5c89a457eba8de73407bd9d13",
        ),
        (
            "c9s6/chart.json",
            "d3844f89a1b909d5affe0c50ad8e32ca8aaf4a6cf0767f21ac98b2e4a56436b3",
        ),
        (
            "c9s6/cutscene-bgm.ogg",
            "91be5ef2315ce6b2dc1f39c9b92925cc96297be61f2430895736030a1ba354cf",
        ),
        (
            "c9s6/cutscene-image.ogg",
            "27e32df227fe5b3e44a98181da8ce9abeaff1969999aed9473db5796ab5cbb9d",
        ),
        (
            "c9s6/cutscene-typing.ogg",
            "09f7eed459102b398a2fc9b69d42ac92f77766b861e09c7c4cf08c1e43c2f2cb",
        ),
        (
            "c9s6/index.json",
            "180fbf920846211b35620f963b94dbe24af7a095452e8bdc40a03352eeaa7a5b",
        ),
        (
            "c9s6/loop.mp4",
            "3b5adc383a1e7941d7573eb5acbd7dcd228209a1c51af3f3c950da6da1d79e5f",
        ),
        (
            "c9s6/music.ogg",
            "282002f05811bbb6d2f3d4efe10eb5d2316b966dde8c850f6de669d4efe3a6df",
        ),
        (
            "collection-art/blackhole0.jpg",
            "2dbdeb65d251811d6235bd30a651d0d041c32460c5816aeed36fc063b6e3d253",
        ),
        (
            "collection-art/blackhole0.png",
            "e1cc81dbbd1f49cbc426d5d95fee7663e1325264ec286d186d333a0468537e52",
        ),
        (
            "collection-art/blackhole0-tile.png",
            "0a9bf20f5563913ebcab00335fca2a3341682bc954ae8c2aaa59b17844839658",
        ),
    ];

    fn inputs() -> Option<Inputs> {
        let open = |name: &str| fixture(name).map(|b| c9s::decrypt(&b).unwrap());
        let meta: Value = serde_json::from_slice(&fixture("meta.json")?).ok()?;
        let scene = |n: usize| -> Option<(String, Vec<u8>)> {
            Some((
                meta["scenes"][n].as_str()?.to_string(),
                fixture(&format!("level{n}"))?,
            ))
        };
        Some(Inputs {
            art: open("art/blackhole0.bundle")?,
            tile: open("c9s/tile.bundle")?,
            chart: open("c9s/chart.bundle")?,
            music: open("c9s/music.bundle")?,
            background: open("c9s/background.bundle")?,
            cutscene: open("c9s/scene.bundle")?,
            metadata: fixture("global-metadata.dat")?,
            level0: fixture("level0")?,
            scenes: vec![scene(28)?, scene(30)?],
            deduction: Vec::new(),
            texts: texts::Sources::default(),
        })
    }

    /// The web UI's rule: each overlay entry replaces the fields it gives on the entry with its id, or is appended
    fn merge(generic: &[Value], overlay: &[Value]) -> Vec<Value> {
        let mut merged = generic.to_vec();
        for patch in overlay {
            match merged.iter_mut().find(|e| e["id"] == patch["id"]) {
                Some(entry) => {
                    for (field, value) in patch.as_object().unwrap() {
                        entry[field] = value.clone();
                    }
                }
                None => merged.push(patch.clone()),
            }
        }
        merged
    }

    /// Missing and null mean the same; entries compared by id
    fn normalized(entries: Vec<Value>) -> BTreeMap<String, BTreeMap<String, Value>> {
        entries
            .into_iter()
            .map(|e| {
                let fields = e
                    .as_object()
                    .unwrap()
                    .iter()
                    .filter(|(_, v)| !v.is_null())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                (e["id"].as_str().unwrap().to_string(), fields)
            })
            .collect()
    }

    #[test]
    fn writes_what_r2_holds_and_an_overlay_that_restores_the_live_manifest() {
        let (Some(inputs), Some(live)) = (inputs(), fixture("collection-art-index.json")) else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("phi-specials-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let report = write_all(&inputs, &dir).unwrap();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        for (key, sha) in LIVE {
            let body = fs::read(dir.join(key)).unwrap();
            assert_eq!(format!("{:x}", Sha256::digest(&body)), sha, "{key}");
        }
        let mut written = report.files.clone();
        written.sort();
        let mut owned: Vec<String> = LIVE.iter().map(|(k, _)| k.to_string()).collect();
        owned.push(OVERLAY.to_string());
        owned.sort();
        assert_eq!(written, owned);

        let overlay: Value = serde_json::from_slice(&fs::read(dir.join(OVERLAY)).unwrap()).unwrap();
        let overlay = overlay["entries"].as_array().unwrap().clone();
        let ids: Vec<&str> = overlay.iter().map(|e| e["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            ["blackhole0", "nizhidaoma1", "poyidomegino0", "poyidomejiu0"]
        );
        let live: Value = serde_json::from_slice(&live).unwrap();
        let live = live["entries"].as_array().unwrap().clone();
        // What phi-unpackd now writes: the live manifest without the passcode entry and the fields this tool gives
        let generic: Vec<Value> = live
            .iter()
            .filter(|e| e["id"] != art::ID)
            .map(|e| {
                let mut e = e.clone();
                let fields = e.as_object_mut().unwrap();
                fields.remove("before");
                fields.remove("tile");
                e
            })
            .collect();
        assert_eq!(normalized(merge(&generic, &overlay)), normalized(live));

        let (files, manifests) = uploads(&dir).unwrap();
        let keys = |list: &[Upload]| {
            list.iter()
                .map(|u| (u.key.clone(), u.mime))
                .collect::<Vec<_>>()
        };
        assert_eq!(files.len(), 11);
        assert!(keys(&files).contains(&("c9s6/loop.mp4".into(), "video/mp4")));
        assert!(keys(&files).contains(&("collection-art/blackhole0.jpg".into(), "image/jpeg")));
        assert_eq!(
            keys(&manifests),
            [
                (c9s::INDEX.to_string(), "application/json"),
                (OVERLAY.to_string(), "application/json")
            ]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn publishes_nothing_from_a_tree_extract_did_not_write() {
        let dir = std::env::temp_dir().join(format!("phi-specials-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(uploads(&dir).is_err());
        fs::create_dir_all(dir.join(c9s::DIR)).unwrap();
        fs::write(dir.join(c9s::INDEX), b"{}").unwrap();
        assert!(uploads(&dir)
            .err()
            .unwrap()
            .contains("collection-art/blackhole0.png is missing"));
        let _ = fs::remove_dir_all(&dir);
    }
}
