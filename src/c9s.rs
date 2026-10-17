//! Chapter 9's secret addressables, `c9s.*`: AES-256-CBC bundles that open with the passcode the Phase 1 map's
//! locked tile asks for (`C9SecretAssetBundleProvider`). The passcode plays a cutscene (`c9s.Qf9C6hd3`, see
//! `cutscene`), puts a new tile on the map (`c9s.zFzUbVcZ`, see `tile`) and starts the hidden song C9S6
//! (`C9S6Unlock.StartC9S6`), written as `c9s6/chart.json`, `c9s6/music.ogg` and `c9s6/background.png`, a Phira pack
//! `c9s6/C9S6-SP.pez` and a `c9s6/index.json` manifest.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use phi_unpackd::pez;
use serde_json::{json, Value};
use sha2::{Digest, Sha512};

pub const DIR: &str = "c9s6";
pub const INDEX: &str = "c9s6/index.json";
pub const PACK: &str = "c9s6/C9S6-SP.pez";
/// The game knows the song only as C9S6: that names its files, in R2 and in the pack
pub const ID: &str = "C9S6";
/// Its title is not in the game's text. The game gives it no level or credits either: it is filed as the April Fools
/// specials are (`SP`, difficulty `?`), and unknown credits are empty, as publish-fools.ts leaves them
pub const NAME: &str = "True home, True world";
pub const LEVEL: &str = "SP";
pub const DIFFICULTY: &str = "?";
pub const CUTSCENE: &str = "c9s.Qf9C6hd3";
pub const TILE: &str = "c9s.zFzUbVcZ";
/// Key and IV are its SHA-512's first 32 and next 16 bytes
const PASSCODE: &str =
    "Backward, go backward, turn back to the antemundane realm, go back to the -";
const PREFIX: &str = "c9s.";
/// `StartC9S6`'s chart and music keys, and the image its level effect loads (`C9S6LevelEffect.Awake`)
const SONG: [(&str, Part); 3] = [
    ("c9s.ilBx0rGL", Part::Chart),
    ("c9s.MYy0ioG2", Part::Music),
    ("c9s.U8WX0Xsd", Part::Background),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Chart,
    Music,
    Background,
}

impl Part {
    pub fn path(self) -> &'static str {
        match self {
            Part::Chart => "c9s6/chart.json",
            Part::Music => "c9s6/music.ogg",
            Part::Background => "c9s6/background.png",
        }
    }
}

/// The catalog key of a part of the song
pub fn part_key(part: Part) -> &'static str {
    SONG.iter()
        .find(|(_, p)| *p == part)
        .map(|(key, _)| *key)
        .unwrap_or_default()
}

pub fn song_part(key: &str) -> Option<Part> {
    SONG.iter().find(|(k, _)| *k == key).map(|(_, part)| *part)
}

pub fn song_parts() -> usize {
    SONG.len()
}

pub fn is_encrypted(key: &str) -> bool {
    key.starts_with(PREFIX)
}

/// The scene the passcode plays before the song
pub fn is_cutscene(key: &str) -> bool {
    key == CUTSCENE
}

/// The prefab that replaces the map's locked tile once the passcode is given
pub fn is_tile(key: &str) -> bool {
    key == TILE
}

pub fn pack_info() -> String {
    pez::info_text(ID, NAME, LEVEL, DIFFICULTY, "", "", "")
}

/// What `c9s6/index.json` says of the song's own files
#[derive(Clone, Debug, PartialEq)]
pub struct Song {
    pub lines: usize,
    pub bpm: f64,
    /// tap, drag, hold, flick
    pub notes: [u32; 4],
    pub width: u32,
    pub height: u32,
}

/// The chart's judge line count and BPM (the first line's, should they differ)
pub fn chart_shape(chart: &[u8]) -> Result<(usize, f64), String> {
    let chart: Value = serde_json::from_slice(chart).map_err(|e| e.to_string())?;
    let lines = chart["judgeLineList"]
        .as_array()
        .ok_or("chart has no judge lines")?;
    let bpm = lines
        .first()
        .and_then(|line| line["bpm"].as_f64())
        .ok_or("chart has no bpm")?;
    Ok((lines.len(), bpm))
}

/// `c9s6/index.json`; `cutscene` is null when the scene could not be read
pub fn index(song: &Song, cutscene: Value) -> Result<Vec<u8>, String> {
    let [tap, drag, hold, flick] = song.notes;
    serde_json::to_vec(&json!({
        "v": 1,
        "name": NAME,
        "level": LEVEL,
        "difficulty": DIFFICULTY,
        "chart": {
            "file": Part::Chart.path(),
            "lines": song.lines,
            "bpm": song.bpm,
            "notes": { "tap": tap, "drag": drag, "hold": hold, "flick": flick },
        },
        "music": { "file": Part::Music.path() },
        "background": { "file": Part::Background.path(), "width": song.width, "height": song.height },
        "pack": { "file": PACK },
        "cutscene": cutscene,
    }))
    .map_err(|e| e.to_string())
}

pub fn decrypt(bundle: &[u8]) -> Result<Vec<u8>, String> {
    let hash = Sha512::digest(PASSCODE.as_bytes());
    cbc::Decryptor::<aes::Aes256>::new_from_slices(&hash[..32], &hash[32..48])
        .map_err(|e| e.to_string())?
        .decrypt_padded_vec_mut::<Pkcs7>(bundle)
        .map_err(|_| "the passcode does not decrypt it".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypts_what_the_passcode_encrypted() {
        use aes::cipher::BlockEncryptMut;
        let hash = Sha512::digest(PASSCODE.as_bytes());
        let plain = b"UnityFS\0\0\0\0\x085.x.x\0".repeat(3);
        let sealed = cbc::Encryptor::<aes::Aes256>::new_from_slices(&hash[..32], &hash[32..48])
            .unwrap()
            .encrypt_padded_vec_mut::<Pkcs7>(&plain);
        assert_eq!(decrypt(&sealed).unwrap(), plain);
        assert!(decrypt(&plain[..32]).is_err());
        assert!(decrypt(&sealed[..sealed.len() - 1]).is_err());
    }

    #[test]
    fn knows_the_secret_keys() {
        assert!(is_encrypted("c9s.vK7mQp2H"));
        assert!(!is_encrypted("Assets/Chapter9/Arts/thelier0.jpg"));
        assert_eq!(song_part("c9s.ilBx0rGL"), Some(Part::Chart));
        assert_eq!(song_part("c9s.U8WX0Xsd"), Some(Part::Background));
        assert_eq!(song_part("c9s.test"), None);
        assert_eq!(Part::Music.path(), format!("{DIR}/music.ogg"));
        assert!(is_cutscene("c9s.Qf9C6hd3") && !is_cutscene("c9s.zFzUbVcZ"));
        assert!(is_tile("c9s.zFzUbVcZ") && !is_tile("c9s.vK7mQp2H"));
        assert_eq!(PACK, format!("{DIR}/{ID}-{LEVEL}.pez"));
        assert_eq!(INDEX, format!("{DIR}/index.json"));
    }

    #[test]
    fn names_the_pack_after_the_song() {
        assert_eq!(
            pack_info(),
            "#\nName: True home, True world\nSong: C9S6.ogg\nPicture: C9S6.png\nChart: C9S6.json\nLevel: SP Lv.?\nComposer: \nIllustrator: \nCharter: "
        );
    }

    #[test]
    fn writes_the_index_with_sorted_keys() {
        let song = Song {
            lines: 24,
            bpm: 220.0,
            notes: [115, 262, 35, 46],
            width: 256,
            height: 135,
        };
        assert_eq!(
            String::from_utf8(index(&song, Value::Null).unwrap()).unwrap(),
            concat!(
                r#"{"background":{"file":"c9s6/background.png","height":135,"width":256},"#,
                r#""chart":{"bpm":220.0,"file":"c9s6/chart.json","lines":24,"notes":{"drag":262,"flick":46,"hold":35,"tap":115}},"#,
                r#""cutscene":null,"difficulty":"?","level":"SP","music":{"file":"c9s6/music.ogg"},"name":"True home, True world","#,
                r#""pack":{"file":"c9s6/C9S6-SP.pez"},"v":1}"#
            )
        );
        assert_eq!(
            chart_shape(br#"{"formatVersion":3,"judgeLineList":[{"bpm":220.0},{"bpm":110}]}"#)
                .unwrap(),
            (2, 220.0)
        );
        assert!(chart_shape(br#"{"formatVersion":3,"judgeLineList":[]}"#).is_err());
        assert!(chart_shape(br#"{"formatVersion":3}"#).is_err());
    }
}
