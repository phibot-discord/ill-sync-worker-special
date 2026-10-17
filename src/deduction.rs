//! Chapter 9's Deduction, the scene played before AfterDeduction (`Assets/Phigros2/Chapter9/Deduction/Deduction.unity`).
//! Its `C9DeductionControl` types pages of lines over full-frame illustrations, one tap per line, with a looping
//! ambience, a typing sound and a sound for some of the transitions. Written as `deduction/<layer>.{jpg,png}`,
//! `deduction/gradient.png`, `deduction/*.ogg` and `deduction/index.json`.

use phi_unpackd::audio;
use phi_unpackd::collection::{self, Text};
use phi_unpackd::cursor::Cursor;
use phi_unpackd::texture::{self, Texture};
use phi_unpackd::unity::{self, Assets, PPtr};
use phi_unpackd::unityfs::Node;
use serde_json::{json, Value};
use std::collections::HashSet;

pub const DIR: &str = "deduction";
pub const INDEX: &str = "deduction/index.json";
pub const SCENE: &str = "Assets/Phigros2/Chapter9/Deduction/Deduction.unity";
const CONTROL: &str = "C9DeductionControl";
const IMAGE: &str = "Image";
const GRADIENT: &str = "deduction/gradient.png";
const JPEG_QUALITY: u8 = 90;
const NULL: PPtr = PPtr { file: 0, path: 0 };

/// The control's image fields, in field order: the names the export and the page visuals use
pub const LAYERS: [&str; 9] = [
    "img1", "img2", "img3", "img3_1", "img3_2", "img3_3", "img3_4", "img3_5", "img3_6",
];

/// The control's single sound fields after `typingSE`, in field order
pub const SOUNDS: [&str; 5] = ["ambient", "img2SE", "whiteToImg3SE", "img3_1SE", "img3_2SE"];

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// In `collection::LANGS` order, line breaks cleaned
    pub lines: Vec<Text>,
    pub visual: i32,
}

/// The fields of `C9DeductionControl` the export uses
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    pub pages: Vec<Page>,
    pub layers: [PPtr; 9],
    pub typing: Vec<PPtr>,
    pub sounds: [PPtr; 5],
    pub gradient: PPtr,
    pub fade_in: f32,
    pub fade_out: f32,
}

fn count(c: &mut Cursor, what: &str) -> Result<usize, String> {
    let n = c.i32()?;
    usize::try_from(n)
        .ok()
        .filter(|n| *n <= 10_000)
        .ok_or_else(|| format!("{what} count {n}"))
}

/// A `C9DeductionControl` object, to its last byte
pub fn parse_control(object: &[u8]) -> Result<Control, String> {
    let (_, _, mut c) = unity::behaviour_header(object)?;
    let mut pages = Vec::new();
    for _ in 0..count(&mut c, "page")? {
        let mut lines = Vec::new();
        for _ in 0..count(&mut c, "line")? {
            let _code = c.string()?;
            let mut text = Text::default();
            for slot in &mut text {
                *slot = collection::clean(&c.string()?);
            }
            lines.push(text);
        }
        let visual = c.i32()?;
        pages.push(Page { lines, visual });
    }
    let mut layers = [NULL; 9];
    for layer in &mut layers {
        *layer = PPtr::read(&mut c)?;
    }
    let typing = (0..count(&mut c, "typing sound")?)
        .map(|_| PPtr::read(&mut c))
        .collect::<Result<_, _>>()?;
    let mut sounds = [NULL; 5];
    for sound in &mut sounds {
        *sound = PPtr::read(&mut c)?;
    }
    let _vignette = PPtr::read(&mut c)?;
    let gradient = PPtr::read(&mut c)?;
    // textScene, textBox, storyFont, cutOut, shaderVariantCollection, levels
    c.bytes(12 * 6)?;
    let fade_in = c.f32()?;
    let fade_out = c.f32()?;
    unity::skip_curve(&mut c)?;
    unity::skip_curve(&mut c)?;
    unity::end(&c, object, CONTROL)?;
    Ok(Control {
        pages,
        layers,
        typing,
        sounds,
        gradient,
        fade_in,
        fade_out,
    })
}

/// How an AudioSource plays its clip, as the scene sets it
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playback {
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
}

/// An AudioSource's clip and playback settings
pub fn audio_source(object: &[u8]) -> Result<(PPtr, Playback), String> {
    let mut c = Cursor::new(object);
    let _game_object = PPtr::read(&mut c)?;
    c.u8()?;
    c.align(4);
    let _mixer = PPtr::read(&mut c)?;
    let clip = PPtr::read(&mut c)?;
    let _play_on_awake = c.u8()?;
    c.align(4);
    let volume = c.f32()?;
    let pitch = c.f32()?;
    let looping = c.u8()? != 0;
    Ok((
        clip,
        Playback {
            volume,
            pitch,
            looping,
        },
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    /// The AudioClip's name
    pub clip: String,
    pub playback: Playback,
    pub ogg: Vec<u8>,
}

pub struct Layer {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    /// The file below the output root and its bytes: JPEG for an opaque texture, PNG for one with alpha
    pub file: (String, Vec<u8>),
}

pub struct Deduction {
    pub control: Control,
    pub layers: Vec<Layer>,
    pub gradient: Option<(u32, u32, Vec<u8>)>,
    /// One per field of `SOUNDS`
    pub sounds: Vec<Option<Sound>>,
    /// One per clip the typing sources play (several sources can share one), in field order
    pub typing: Vec<Option<Sound>>,
}

/// The scene's nodes: its level, its shared assets with their streamed data, and globalgamemanagers.assets. Only a
/// missing or unreadable control fails it; a layer or sound that cannot be read is left out with a warning
pub fn read(nodes: &[Node], warnings: &mut Vec<String>) -> Result<Deduction, String> {
    let assets = Assets::new(nodes)?;
    let object = match assets.behaviours(CONTROL).as_slice() {
        [one] => *one,
        [] => return Err(format!("no {CONTROL}")),
        many => return Err(format!("{} {CONTROL}s", many.len())),
    };
    let control = parse_control(object.bytes)?;
    let from = object.file;
    let mut warn = |what: &str, err: String| warnings.push(format!("deduction {what}: {err}"));
    let mut layers = Vec::new();
    for (name, image) in LAYERS.iter().zip(control.layers) {
        match image_texture(&assets, from, image).and_then(|tex| layer(name, tex)) {
            Ok(layer) => layers.push(layer),
            Err(err) => warn(name, err),
        }
    }
    let gradient = image_texture(&assets, from, control.gradient)
        .and_then(|tex| Ok((tex.width, tex.height, texture::encode_png(&tex)?)))
        .map_err(|e| warn("bottomGradient", e))
        .ok();
    let sounds = SOUNDS
        .iter()
        .zip(control.sounds)
        .map(|(name, source)| sound(&assets, from, source).map_err(|e| warn(name, e)).ok())
        .collect();
    let mut seen = HashSet::new();
    let mut typing = Vec::new();
    for source in &control.typing {
        let found = assets
            .get(from, *source, unity::AUDIO_SOURCE)
            .and_then(|s| audio_source(s.bytes).map(|(clip, _)| (s.file, clip)));
        match found {
            Ok((file, clip)) => {
                if seen.insert((assets.file_name(file, clip)?.to_string(), clip.path)) {
                    typing.push(
                        sound(&assets, from, *source)
                            .map_err(|e| warn("typingSE", e))
                            .ok(),
                    );
                }
            }
            Err(err) => warn("typingSE", err),
        }
    }
    Ok(Deduction {
        control,
        layers,
        gradient,
        sounds,
        typing,
    })
}

fn image_texture(assets: &Assets, from: usize, image: PPtr) -> Result<Texture, String> {
    let image = assets.get(from, image, unity::MONO_BEHAVIOUR)?;
    let class = assets.script_class(image)?;
    if class != IMAGE {
        return Err(format!("a {class}, not an {IMAGE}"));
    }
    let sprite = unity::image_sprite(image.bytes)?;
    if sprite.is_null() {
        return Err("shows no sprite".into());
    }
    let sprite = assets.get(image.file, sprite, unity::SPRITE)?;
    let tex = assets.get(
        sprite.file,
        unity::sprite_texture(sprite.bytes)?,
        unity::TEXTURE_2D,
    )?;
    texture::texture_object(tex.bytes, assets.nodes())
}

fn opaque(tex: &Texture) -> bool {
    match tex.color {
        image::ColorType::Rgba8 => tex.pixels.chunks_exact(4).all(|p| p[3] == 255),
        _ => true,
    }
}

fn layer(name: &'static str, tex: Texture) -> Result<Layer, String> {
    let (width, height) = (tex.width, tex.height);
    let file = if opaque(&tex) {
        let rgb = tex.into_rgb().ok_or("not an 8-bit texture")?;
        (
            format!("{DIR}/{name}.jpg"),
            texture::encode_rgb_jpeg(&rgb, JPEG_QUALITY)?,
        )
    } else {
        (format!("{DIR}/{name}.png"), texture::encode_png(&tex)?)
    };
    Ok(Layer {
        name,
        width,
        height,
        file,
    })
}

fn sound(assets: &Assets, from: usize, source: PPtr) -> Result<Sound, String> {
    let source = assets.get(from, source, unity::AUDIO_SOURCE)?;
    let (clip, playback) = audio_source(source.bytes)?;
    let clip = assets.get(source.file, clip, unity::AUDIO_CLIP)?;
    let clip = unity::audio_clip(clip.bytes)?;
    let fsb = clip.data(assets.resource(&clip.source)?)?;
    let ogg = audio::fsb_to_ogg(fsb).map_err(|e| format!("{}: {e}", clip.name))?;
    Ok(Sound {
        clip: clip.name,
        playback,
        ogg,
    })
}

/// An f32 as the shortest decimal that reads back as it (2.0, 1.5, 0.1), not its exact f64 value
fn decimal(x: f32) -> f64 {
    x.to_string().parse().unwrap_or(f64::from(x))
}

fn sound_file(name: &str) -> String {
    format!("{DIR}/{name}.ogg")
}

impl Deduction {
    fn typing_file(&self, n: usize) -> String {
        if self.typing.len() == 1 {
            sound_file("typing")
        } else {
            sound_file(&format!("typing-{}", n + 1))
        }
    }

    /// Every file it writes, by path below the output root
    pub fn files(&self) -> Vec<(String, &[u8])> {
        let mut files: Vec<(String, &[u8])> = self
            .layers
            .iter()
            .map(|layer| (layer.file.0.clone(), layer.file.1.as_slice()))
            .collect();
        if let Some((_, _, png)) = &self.gradient {
            files.push((GRADIENT.to_string(), png.as_slice()));
        }
        for (name, sound) in SOUNDS.iter().zip(&self.sounds) {
            if let Some(sound) = sound {
                files.push((sound_file(name), sound.ogg.as_slice()));
            }
        }
        for (n, sound) in self.typing.iter().enumerate() {
            if let Some(sound) = sound {
                files.push((self.typing_file(n), sound.ogg.as_slice()));
            }
        }
        files
    }

    fn sound_json(file: String, sound: &Sound) -> Value {
        json!({
            "file": file,
            "clip": sound.clip,
            "volume": decimal(sound.playback.volume),
            "pitch": decimal(sound.playback.pitch),
            "loop": sound.playback.looping,
        })
    }

    /// `deduction/index.json`: the pages, the layers and sounds by the control's field names, and the scene's fades
    pub fn index(&self) -> Result<Vec<u8>, String> {
        let pages: Vec<Value> = self
            .control
            .pages
            .iter()
            .map(|page| {
                json!({
                    "visual": page.visual,
                    "lines": page.lines.iter().map(collection::by_lang).collect::<Vec<_>>(),
                })
            })
            .collect();
        let layers: serde_json::Map<String, Value> = self
            .layers
            .iter()
            .map(|layer| {
                (
                    layer.name.to_string(),
                    json!({ "file": layer.file.0, "width": layer.width, "height": layer.height }),
                )
            })
            .collect();
        let sounds: serde_json::Map<String, Value> = SOUNDS
            .iter()
            .zip(&self.sounds)
            .filter_map(|(name, sound)| {
                Some((
                    name.to_string(),
                    Self::sound_json(sound_file(name), sound.as_ref()?),
                ))
            })
            .collect();
        let typing: Vec<Value> = self
            .typing
            .iter()
            .enumerate()
            .filter_map(|(n, sound)| Some(Self::sound_json(self.typing_file(n), sound.as_ref()?)))
            .collect();
        let body = json!({
            "v": 1,
            "scene": SCENE,
            "pages": pages,
            "layers": layers,
            "gradient": self.gradient.as_ref().map(|(w, h, _)| json!({ "file": GRADIENT, "width": w, "height": h })),
            "sounds": sounds,
            "typing": typing,
            "fade": { "in": decimal(self.control.fade_in), "out": decimal(self.control.fade_out) },
        });
        serde_json::to_vec(&body).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{header, p, s};

    fn line(out: &mut Vec<u8>, texts: [&str; 5]) {
        s(out, "");
        for text in texts {
            s(out, text);
        }
    }

    fn curve(out: &mut Vec<u8>) {
        out.extend(2i32.to_le_bytes());
        out.extend(vec![0u8; 2 * 28]);
        for v in [2i32, 2, 4] {
            out.extend(v.to_le_bytes());
        }
    }

    fn control_object() -> Vec<u8> {
        let mut out = Vec::new();
        header(&mut out, 7, (1, 916));
        out.extend(2i32.to_le_bytes());
        out.extend(1i32.to_le_bytes());
        line(&mut out, ["天空", "天空", "Sky", "空", "하늘"]);
        out.extend(0i32.to_le_bytes());
        out.extend(2i32.to_le_bytes());
        line(&mut out, ["一", "一", "One", "一", "하나"]);
        line(&mut out, ["二", "二", "Two", "二", "둘"]);
        out.extend(1i32.to_le_bytes());
        for n in 0..9 {
            p(&mut out, 0, 70 + n);
        }
        out.extend(3i32.to_le_bytes());
        for n in 0..3 {
            p(&mut out, 0, 35 + n);
        }
        for n in 0..5 {
            p(&mut out, 0, 30 + n);
        }
        p(&mut out, 0, 78);
        p(&mut out, 0, 79);
        for n in 0..6 {
            p(&mut out, n, 60 + i64::from(n));
        }
        out.extend(0.5f32.to_le_bytes());
        out.extend(1.5f32.to_le_bytes());
        curve(&mut out);
        curve(&mut out);
        out
    }

    #[test]
    fn reads_a_control_to_its_last_byte() {
        let control = parse_control(&control_object()).unwrap();
        assert_eq!(control.pages.len(), 2);
        assert_eq!(control.pages[0].visual, 0);
        assert_eq!(control.pages[1].visual, 1);
        assert_eq!(control.pages[1].lines.len(), 2);
        assert_eq!(control.pages[1].lines[1][2], "Two");
        assert_eq!(control.layers[8].path, 78);
        assert_eq!(control.typing.len(), 3);
        assert_eq!(control.sounds[0].path, 30);
        assert_eq!(control.gradient.path, 79);
        assert_eq!((control.fade_in, control.fade_out), (0.5, 1.5));
        let mut short = control_object();
        short.pop();
        assert!(parse_control(&short).is_err());
    }

    #[test]
    fn reads_an_audio_source() {
        let mut out = Vec::new();
        p(&mut out, 0, 12);
        out.extend([1u8, 0, 0, 0]);
        p(&mut out, 0, 0);
        p(&mut out, 2, 99);
        out.extend([1u8, 0, 0, 0]);
        out.extend(0f32.to_le_bytes());
        out.extend(1f32.to_le_bytes());
        out.push(1);
        let (clip, playback) = audio_source(&out).unwrap();
        assert_eq!((clip.file, clip.path), (2, 99));
        assert_eq!(
            playback,
            Playback {
                volume: 0.0,
                pitch: 1.0,
                looping: true
            }
        );
    }
}
