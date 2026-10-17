//! The scene the Phase 1 map's passcode plays before C9S6 (catalog key `c9s.Qf9C6hd3`). Its `C9StoryControl` shows
//! pages of lines over a looping video, brings in an image from a set page and can fade to white at a set line, with a
//! background loop, a sound for the image and a typing sound. Everything is reached from that component through its
//! references. Written as `c9s6/loop.mp4` (the video, byte for byte), `c9s6/cutscene-*.ogg` and the `cutscene` of
//! `c9s6/index.json`.

use crate::art::Sprites;
use phi_unpackd::audio;
use phi_unpackd::collection::{self, Text};
use phi_unpackd::collection_art;
use phi_unpackd::cursor::Cursor;
use phi_unpackd::pv;
use phi_unpackd::unity::{self, Assets, PPtr};
use phi_unpackd::unityfs::Node;
use serde_json::{json, Value};
use std::collections::HashSet;

pub const VIDEO: &str = "c9s6/loop.mp4";
const BGM: &str = "c9s6/cutscene-bgm.ogg";
const IMAGE_SOUND: &str = "c9s6/cutscene-image.ogg";
const STORY: &str = "C9StoryControl";
const IMAGE: &str = "Image";

/// The fields of `C9StoryControl` the export uses
#[derive(Clone, Debug, PartialEq)]
pub struct Story {
    /// Each page's lines, in `collection::LANGS` order with line breaks cleaned
    pub pages: Vec<Vec<Text>>,
    /// The page the image comes in on
    pub image_page: i32,
    pub video: PPtr,
    pub image: PPtr,
    pub image_sound: PPtr,
    pub typing: Vec<PPtr>,
    pub bgm: PPtr,
    /// Seconds the image takes to come in
    pub entrance: f32,
    pub video_fade_in: f32,
    pub fade_to_white: Cue,
}

/// `C9StoryControl.fadeToWhite`: at a line, fade to white over `duration` and back over `fade_out`
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub enabled: bool,
    pub page: i32,
    pub line: i32,
    pub duration: f32,
    pub fade_out: f32,
    pub auto_next: bool,
}

fn count(c: &mut Cursor, what: &str) -> Result<usize, String> {
    let n = c.i32()?;
    usize::try_from(n)
        .ok()
        .filter(|n| *n <= 10_000)
        .ok_or_else(|| format!("{what} count {n}"))
}

/// A `C9StoryControl` object, to its last byte
pub fn parse_story(object: &[u8]) -> Result<Story, String> {
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
        pages.push(lines);
    }
    let image_page = c.i32()?;
    let video = PPtr::read(&mut c)?;
    let image = PPtr::read(&mut c)?;
    let _shader = PPtr::read(&mut c)?;
    let image_sound = PPtr::read(&mut c)?;
    let typing = (0..count(&mut c, "typing sound")?)
        .map(|_| PPtr::read(&mut c))
        .collect::<Result<_, _>>()?;
    let bgm = PPtr::read(&mut c)?;
    // vignette, bottomGradient, screenFade, screenWhite, textScene, textBox, storyFont, cutOut
    c.bytes(8 * 12)?;
    let entrance = c.f32()?;
    unity::skip_curve(&mut c)?;
    let video_fade_in = c.f32()?;
    unity::skip_curve(&mut c)?;
    let enabled = c.bool()?;
    c.align(4);
    let page = c.i32()?;
    let line = c.i32()?;
    let duration = c.f32()?;
    let fade_out = c.f32()?;
    let auto_next = c.bool()?;
    c.align(4);
    unity::skip_curve(&mut c)?;
    unity::skip_curve(&mut c)?;
    let _post_effects = PPtr::read(&mut c)?;
    unity::end(&c, object, STORY)?;
    Ok(Story {
        pages,
        image_page,
        video,
        image,
        image_sound,
        typing,
        bgm,
        entrance,
        video_fade_in,
        fade_to_white: Cue {
            enabled,
            page,
            line,
            duration,
            fade_out,
            auto_next,
        },
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    /// The AudioClip's name
    pub clip: String,
    pub ogg: Vec<u8>,
}

pub struct Cutscene {
    pub story: Story,
    /// The VideoClip and its MP4
    pub video: Option<(pv::Clip, Vec<u8>)>,
    pub bgm: Option<Sound>,
    pub image_sound: Option<Sound>,
    /// One per clip the typing sources play (several sources can share one), in field order
    pub typing: Vec<Option<Sound>>,
    /// The serialized file and path ID of the sprite the image shows
    pub sprite: Option<(String, i64)>,
}

/// The cutscene in a decrypted scene bundle. Only a missing or unreadable `C9StoryControl` fails it; a video, sound or
/// image that cannot be read is left out with a warning
pub fn read(nodes: &[Node], warnings: &mut Vec<String>) -> Result<Cutscene, String> {
    let assets = Assets::new(nodes)?;
    let control = match assets.behaviours(STORY).as_slice() {
        [one] => *one,
        [] => return Err(format!("no {STORY}")),
        many => return Err(format!("{} {STORY}s", many.len())),
    };
    let story = parse_story(control.bytes)?;
    let from = control.file;
    let mut warn = |what: &str, err: String| warnings.push(format!("cutscene {what}: {err}"));
    let video = video(&assets, from, story.video)
        .map_err(|e| warn("video", e))
        .ok();
    let bgm = clip_of(&assets, from, story.bgm)
        .and_then(|clip| decode(&assets, clip))
        .map_err(|e| warn("bgm", e))
        .ok();
    let image_sound = clip_of(&assets, from, story.image_sound)
        .and_then(|clip| decode(&assets, clip))
        .map_err(|e| warn("image sound", e))
        .ok();
    let mut seen = HashSet::new();
    let mut clips = Vec::new();
    for source in &story.typing {
        match clip_of(&assets, from, *source) {
            Ok(clip) if seen.insert((clip.file, clip.path)) => clips.push(clip),
            Ok(_) => {}
            Err(err) => warn("typing sound", err),
        }
    }
    let typing = clips
        .into_iter()
        .map(|clip| {
            decode(&assets, clip)
                .map_err(|e| warn("typing sound", e))
                .ok()
        })
        .collect();
    let sprite = sprite(&assets, from, story.image)
        .map_err(|e| warn("image", e))
        .ok();
    Ok(Cutscene {
        story,
        video,
        bgm,
        image_sound,
        typing,
        sprite,
    })
}

fn video(assets: &Assets, from: usize, player: PPtr) -> Result<(pv::Clip, Vec<u8>), String> {
    let player = assets.get(from, player, unity::VIDEO_PLAYER)?;
    let clip = unity::video_player_clip(player.bytes)?;
    let clip = pv::parse_clip(assets.get(player.file, clip, unity::VIDEO_CLIP)?.bytes)?;
    let mp4 = pv::mp4(assets.resource(&clip.source)?, &clip)?.to_vec();
    Ok((clip, mp4))
}

/// The AudioClip an AudioSource plays
fn clip_of<'a>(
    assets: &Assets<'a>,
    from: usize,
    source: PPtr,
) -> Result<unity::Object<'a>, String> {
    let source = assets.get(from, source, unity::AUDIO_SOURCE)?;
    let clip = unity::audio_source_clip(source.bytes)?;
    assets.get(source.file, clip, unity::AUDIO_CLIP)
}

fn decode(assets: &Assets, clip: unity::Object) -> Result<Sound, String> {
    let clip = unity::audio_clip(clip.bytes)?;
    let fsb = clip.data(assets.resource(&clip.source)?)?;
    let ogg = audio::fsb_to_ogg(fsb).map_err(|e| format!("{}: {e}", clip.name))?;
    Ok(Sound {
        clip: clip.name,
        ogg,
    })
}

fn sprite(assets: &Assets, from: usize, image: PPtr) -> Result<(String, i64), String> {
    let image = assets.get(from, image, unity::MONO_BEHAVIOUR)?;
    let class = assets.script_class(image)?;
    if class != IMAGE {
        return Err(format!("a {class}, not an {IMAGE}"));
    }
    let sprite = unity::image_sprite(image.bytes)?;
    if sprite.is_null() {
        return Err("shows no sprite".into());
    }
    Ok((
        assets.file_name(image.file, sprite)?.to_string(),
        sprite.path,
    ))
}

/// An f32 as the shortest decimal that reads back as it (2.0, 1.5, 0.1), not its exact f64 value
fn decimal(x: f32) -> f64 {
    x.to_string().parse().unwrap_or(f64::from(x))
}

impl Cutscene {
    fn typing_file(&self, n: usize) -> String {
        if self.typing.len() == 1 {
            "c9s6/cutscene-typing.ogg".into()
        } else {
            format!("c9s6/cutscene-typing-{}.ogg", n + 1)
        }
    }

    /// Every file it writes, by path below the output root
    pub fn files(&self) -> Vec<(String, &[u8])> {
        let mut files = Vec::new();
        if let Some((_, mp4)) = &self.video {
            files.push((VIDEO.to_string(), mp4.as_slice()));
        }
        for (path, sound) in [(BGM, &self.bgm), (IMAGE_SOUND, &self.image_sound)] {
            if let Some(sound) = sound {
                files.push((path.to_string(), sound.ogg.as_slice()));
            }
        }
        for (n, sound) in self.typing.iter().enumerate() {
            if let Some(sound) = sound {
                files.push((self.typing_file(n), sound.ogg.as_slice()));
            }
        }
        files
    }

    /// The `cutscene` of `c9s6/index.json`. Its image is the collection art whose bundle holds the sprite the scene
    /// shows, among those this run exported
    pub fn json(&self, arts: &[Sprites], warnings: &mut Vec<String>) -> Value {
        let story = &self.story;
        let art = self.sprite.as_ref().and_then(|(file, path)| {
            let found = arts
                .iter()
                .find(|art| art.sprites.contains(&(file.clone(), *path)));
            if found.is_none() {
                warnings.push(format!(
                    "cutscene image: sprite {path} of {file} is in no collection art this run exported"
                ));
            }
            found
        });
        let cue = &story.fade_to_white;
        let typing: Vec<String> = self
            .typing
            .iter()
            .enumerate()
            .filter(|(_, sound)| sound.is_some())
            .map(|(n, _)| self.typing_file(n))
            .collect();
        json!({
            "pages": story
                .pages
                .iter()
                .map(|lines| json!({ "lines": lines.iter().map(collection::by_lang).collect::<Vec<_>>() }))
                .collect::<Vec<_>>(),
            "image": art.map(|art| json!({
                "page": story.image_page,
                "entrance": decimal(story.entrance),
                "file": collection_art::jpg_file(&art.id),
                "original": collection_art::png_file(&art.id),
                "width": art.width,
                "height": art.height,
            })),
            "video": self.video.as_ref().map(|(clip, _)| json!({
                "file": VIDEO,
                "width": clip.width,
                "height": clip.height,
                "fps": clip.fps,
                "frames": clip.frames,
                "fadeIn": decimal(story.video_fade_in),
            })),
            "fadeToWhite": cue.enabled.then(|| json!({
                "page": cue.page,
                "line": cue.line,
                "in": decimal(cue.duration),
                "out": decimal(cue.fade_out),
                "autoNext": cue.auto_next,
            })),
            "sound": {
                "bgm": self.bgm.as_ref().map(|_| BGM),
                "image": self.image_sound.as_ref().map(|_| IMAGE_SOUND),
                "typing": typing,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c9s;
    use crate::testing::{header, p, s};
    use phi_unpackd::unityfs;
    use sha2::{Digest, Sha256};

    fn curve(out: &mut Vec<u8>, keys: usize) {
        out.extend((keys as i32).to_le_bytes());
        out.extend(vec![3u8; keys * 28]);
        for v in [2i32, 2, 4] {
            out.extend(v.to_le_bytes());
        }
    }

    fn line(out: &mut Vec<u8>, texts: [&str; 5]) {
        s(out, "");
        for text in texts {
            s(out, text);
        }
    }

    fn story_object() -> Vec<u8> {
        let mut b = Vec::new();
        header(&mut b, 10, (1, 4));
        b.extend(2i32.to_le_bytes());
        b.extend(1i32.to_le_bytes());
        line(&mut b, ["一\r\n二", "一", "One", "", "하나"]);
        b.extend(2i32.to_le_bytes());
        line(&mut b, ["a", "b", "c", "d", "e"]);
        line(&mut b, ["f", "g", "h", "i", "j"]);
        b.extend(1i32.to_le_bytes());
        p(&mut b, 0, 51);
        p(&mut b, 0, 54);
        p(&mut b, 1, 35);
        p(&mut b, 0, 26);
        b.extend(3i32.to_le_bytes());
        for path in [30, 29, 28] {
            p(&mut b, 0, path);
        }
        p(&mut b, 0, 27);
        for path in 60..68 {
            p(&mut b, 0, path);
        }
        b.extend(2f32.to_le_bytes());
        curve(&mut b, 2);
        b.extend(1.5f32.to_le_bytes());
        curve(&mut b, 0);
        b.extend([1, 0, 0, 0]);
        for v in [1i32, 0] {
            b.extend(v.to_le_bytes());
        }
        b.extend(0.5f32.to_le_bytes());
        b.extend(0.1f32.to_le_bytes());
        b.extend([1, 0, 0, 0]);
        curve(&mut b, 2);
        curve(&mut b, 1);
        p(&mut b, 0, 63);
        b
    }

    #[test]
    fn reads_a_story_to_its_last_byte() {
        let object = story_object();
        let story = parse_story(&object).unwrap();
        assert_eq!(story.pages.len(), 2);
        assert_eq!(
            story.pages[0][0],
            ["一\n二", "一", "One", "", "하나"].map(String::from)
        );
        assert_eq!(story.pages[1][1][4], "j");
        assert_eq!(story.image_page, 1);
        assert_eq!(
            (story.video, story.image, story.image_sound, story.bgm),
            (
                PPtr { file: 0, path: 51 },
                PPtr { file: 0, path: 54 },
                PPtr { file: 0, path: 26 },
                PPtr { file: 0, path: 27 }
            )
        );
        let typing: Vec<i64> = story.typing.iter().map(|t| t.path).collect();
        assert_eq!(typing, [30, 29, 28]);
        assert_eq!((story.entrance, story.video_fade_in), (2.0, 1.5));
        assert_eq!(
            story.fade_to_white,
            Cue {
                enabled: true,
                page: 1,
                line: 0,
                duration: 0.5,
                fade_out: 0.1,
                auto_next: true
            }
        );
        let mut longer = object.clone();
        longer.extend([0u8; 4]);
        assert!(parse_story(&longer).is_err());
        assert!(parse_story(&object[..object.len() - 12]).is_err());
    }

    fn sound(clip: &str) -> Option<Sound> {
        Some(Sound {
            clip: clip.into(),
            ogg: b"OggS".to_vec(),
        })
    }

    #[test]
    fn writes_the_cutscene_with_sorted_keys() {
        let mut cutscene = Cutscene {
            story: parse_story(&story_object()).unwrap(),
            video: Some((
                pv::Clip {
                    name: "LoopBackground".into(),
                    width: 1920,
                    height: 1080,
                    fps: 30.0,
                    frames: 270,
                    audio: false,
                    source: "CAB-1.resource".into(),
                    offset: 0,
                    size: 4,
                },
                b"mp4!".to_vec(),
            )),
            bgm: sound("bgm"),
            image_sound: None,
            typing: vec![sound("type")],
            sprite: Some(("CAB-art".into(), -5)),
        };
        let arts = [Sprites {
            id: "blackhole0".into(),
            width: 2048,
            height: 1152,
            sprites: vec![("CAB-art".into(), -5)],
        }];
        let mut warnings = Vec::new();
        assert_eq!(
            cutscene.json(&arts, &mut warnings).to_string(),
            concat!(
                r#"{"fadeToWhite":{"autoNext":true,"in":0.5,"line":0,"out":0.1,"page":1},"#,
                r#""image":{"entrance":2.0,"file":"collection-art/blackhole0.jpg","height":1152,"original":"collection-art/blackhole0.png","page":1,"width":2048},"#,
                r#""pages":[{"lines":[{"en":"One","ja":"","ko":"하나","zh-CN":"一\n二","zh-TW":"一"}]},"#,
                r#"{"lines":[{"en":"c","ja":"d","ko":"e","zh-CN":"a","zh-TW":"b"},{"en":"h","ja":"i","ko":"j","zh-CN":"f","zh-TW":"g"}]}],"#,
                r#""sound":{"bgm":"c9s6/cutscene-bgm.ogg","image":null,"typing":["c9s6/cutscene-typing.ogg"]},"#,
                r#""video":{"fadeIn":1.5,"file":"c9s6/loop.mp4","fps":30.0,"frames":270,"height":1080,"width":1920}}"#
            )
        );
        assert!(warnings.is_empty());
        let names: Vec<String> = cutscene.files().into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            names,
            [
                "c9s6/loop.mp4",
                "c9s6/cutscene-bgm.ogg",
                "c9s6/cutscene-typing.ogg"
            ]
        );

        cutscene.story.fade_to_white.enabled = false;
        cutscene.video = None;
        cutscene.typing = vec![sound("a"), None, sound("c")];
        cutscene.sprite = Some(("CAB-other".into(), -5));
        let value = cutscene.json(&arts, &mut warnings);
        assert_eq!(value["fadeToWhite"], Value::Null);
        assert_eq!(value["video"], Value::Null);
        assert_eq!(value["image"], Value::Null);
        assert_eq!(
            value["sound"]["typing"],
            json!(["c9s6/cutscene-typing-1.ogg", "c9s6/cutscene-typing-3.ogg"])
        );
        assert_eq!(
            warnings,
            ["cutscene image: sprite -5 of CAB-other is in no collection art this run exported"]
        );
        let names: Vec<String> = cutscene.files().into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            names,
            [
                "c9s6/cutscene-bgm.ogg",
                "c9s6/cutscene-typing-1.ogg",
                "c9s6/cutscene-typing-3.ogg"
            ]
        );
    }

    /// The granule of an Ogg stream's last page, in samples
    fn last_granule(ogg: &[u8]) -> u64 {
        let at = (0..ogg.len() - 4)
            .rev()
            .find(|&i| &ogg[i..i + 4] == b"OggS")
            .unwrap();
        u64::from_le_bytes(ogg[at + 6..at + 14].try_into().unwrap())
    }

    #[test]
    fn reads_the_401_cutscene() {
        let Ok(bundle) = std::fs::read("/tmp/phi-fixtures/c9s/scene.bundle") else {
            return;
        };
        let nodes = unityfs::decode(&c9s::decrypt(&bundle).unwrap()).unwrap();
        let mut warnings = Vec::new();
        let cutscene = read(&nodes, &mut warnings).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let story = &cutscene.story;
        let lines: Vec<usize> = story.pages.iter().map(Vec::len).collect();
        assert_eq!(lines, [1, 1, 2, 1, 1, 2, 2, 2, 2, 1, 1, 4, 2, 1, 1]);
        for (page, line, text) in [
            (
                0,
                0,
                [
                    "黑洞的发难无声无息。",
                    "黑洞的發難無聲無息。",
                    "The black hole's attack came without sound.",
                    "ブラックホールは、何の前触れもなく牙を剥いた。",
                    "블랙홀의 덮침은 소리 없이 다가왔다.",
                ],
            ),
            (
                9,
                0,
                [
                    "Gino：轮到我们了！",
                    "Gino：輪到我們了！",
                    "Gino: Our turn!",
                    "Gino: 次は俺たちの番だ！",
                    "지노: 이제 우리 차례야!",
                ],
            ),
            (
                13,
                0,
                [
                    "Gino：我会来找你的！",
                    "Gino：我會來找你的！",
                    "Gino: I'll find you!",
                    "Gino: 俺が見つけに行く！",
                    "지노: 내가 찾으러 갈게!",
                ],
            ),
            (
                14,
                0,
                [
                    "获得收藏品：答案",
                    "獲得收藏品：答案",
                    "Item collected: The Answer",
                    "コレクション獲得：答え",
                    "컬렉션 획득: 해답",
                ],
            ),
        ] {
            assert_eq!(
                story.pages[page][line],
                text.map(String::from),
                "{page} {line}"
            );
        }
        assert_eq!((story.image_page, story.entrance), (9, 2.0));
        assert_eq!(story.video_fade_in, 1.5);
        assert_eq!(
            story.fade_to_white,
            Cue {
                enabled: true,
                page: 13,
                line: 0,
                duration: 0.5,
                fade_out: 0.5,
                auto_next: true
            }
        );
        let (clip, mp4) = cutscene.video.as_ref().unwrap();
        assert_eq!(
            (
                clip.name.as_str(),
                clip.width,
                clip.height,
                clip.fps,
                clip.frames
            ),
            ("LoopBackground", 1920, 1080, 30.0, 270)
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(mp4)),
            "3b5adc383a1e7941d7573eb5acbd7dcd228209a1c51af3f3c950da6da1d79e5f"
        );
        assert_eq!(
            cutscene.sprite,
            Some((
                "CAB-bef9f1437805ecd4a3a22ea0f70a54e4".to_string(),
                -3217550930489318217
            ))
        );
        // Each sound is the clip its source plays: name, its FSB bytes as UnityPy reads them, its length
        let assets = Assets::new(&nodes).unwrap();
        let fsb = |name: &str| {
            let clip = assets
                .objects()
                .into_iter()
                .filter(|o| o.class_id == unity::AUDIO_CLIP)
                .map(|o| unity::audio_clip(o.bytes).unwrap())
                .find(|c| c.name == name)
                .unwrap();
            let data = clip.data(assets.resource(&clip.source).unwrap()).unwrap();
            format!("{:x}", Sha256::digest(data))
        };
        assert_eq!(cutscene.typing.len(), 1);
        for (sound, name, sha, seconds) in [
            (
                &cutscene.bgm,
                "黑洞音效(故障背景音)",
                "a02a6a790425a5203a2829aeaa5d52083dae9e20c8a85bb17c35ddf1008147c8",
                16.285715,
            ),
            (
                &cutscene.image_sound,
                "黑洞音效(转场)",
                "38e0fb0cd2c919684c388fcbe896293dcfb137890fa9661f1ec9bcf4a49a3e97",
                16.791611,
            ),
            (
                &cutscene.typing[0],
                "文字浮现音效",
                "ba8484697789b87654f612476d551138f0b30c8554bbbaf97e670d670ecc88da",
                0.140136,
            ),
        ] {
            let sound = sound.as_ref().unwrap();
            assert_eq!(sound.clip, name);
            assert_eq!(fsb(name), sha, "{name}");
            assert!(sound.ogg.starts_with(b"OggS"), "{name}");
            let length = last_granule(&sound.ogg) as f64 / 44100.0;
            assert!((length - seconds).abs() < 0.05, "{name}: {length} s");
        }
        let names: Vec<String> = cutscene.files().into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            names,
            [
                "c9s6/loop.mp4",
                "c9s6/cutscene-bgm.ogg",
                "c9s6/cutscene-image.ogg",
                "c9s6/cutscene-typing.ogg"
            ]
        );
    }
}
