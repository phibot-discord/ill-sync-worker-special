//! The collection entry the Phase 1 map's passcode tile opens, `blackhole0`: its art is the passcode-encrypted
//! `c9s.vK7mQp2H` (the game hard-codes the pair in `BuildHeaderImageAddress`) and it opens beside the text
//! (`OpenCollectionVertical`, in code), on the map whose locked tile calls `OpenInputPassword`

use phi_unpackd::collection_art;
use phi_unpackd::extract;
use phi_unpackd::texture;

pub const ID: &str = "blackhole0";
pub const KEY: &str = "c9s.vK7mQp2H";
pub const LAYOUT: &str = "side";
/// The persistent call of the locked tile's button
const PASSCODE_CALL: &str = "OpenInputPassword";

/// An exported art and the sprites its bundle holds (serialized file, path ID), so a scene that shows one of them can
/// be matched to it
#[derive(Clone, Debug, PartialEq)]
pub struct Sprites {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub sprites: Vec<(String, i64)>,
}

/// The art as the ripper writes collection art: lossless PNG, the JPEG the web UI shows, and its sprites
pub struct Art {
    pub png: Vec<u8>,
    pub jpg: Vec<u8>,
    pub sprites: Sprites,
}

/// The decrypted art bundle
pub fn read(bundle: &[u8]) -> Result<Art, String> {
    let (rgb, sprites) = extract::art_rgb(bundle)?;
    Ok(Art {
        png: texture::encode_rgb_png(&rgb)?,
        jpg: texture::encode_rgb_jpeg(&rgb, collection_art::JPEG_QUALITY)?,
        sprites: Sprites {
            id: ID.to_string(),
            width: rgb.width(),
            height: rgb.height(),
            sprites,
        },
    })
}

/// Whether a scene file holds the passcode button's persistent call: the method name as Unity stores a string
pub fn has_passcode_call(scene: &[u8]) -> bool {
    let mut stored = (PASSCODE_CALL.len() as i32).to_le_bytes().to_vec();
    stored.extend(PASSCODE_CALL.as_bytes());
    stored.extend([0u8; 3]);
    extract::find(scene, &stored).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c9s;
    use crate::testing::fixture;
    use phi_unpackd::unityfs;
    use sha2::{Digest, Sha256};

    #[test]
    fn finds_the_map_with_the_passcode_button() {
        let mut scene = vec![1u8; 9];
        scene.extend(17i32.to_le_bytes());
        scene.extend(b"OpenInputPassword\0\0\0");
        assert!(has_passcode_call(&scene));
        let mut other = vec![1u8; 9];
        other.extend(25i32.to_le_bytes());
        other.extend(b"OpenInputPasswordSEPlayer\0\0\0");
        assert!(!has_passcode_call(&other));
        let (Some(level28), Some(level30)) = (fixture("level28"), fixture("level30")) else {
            return;
        };
        assert!(has_passcode_call(&level28));
        assert!(!has_passcode_call(&level30));
    }

    #[test]
    fn decodes_the_passcode_art_of_the_401_apk() {
        let Some(bundle) = fixture("art/blackhole0.bundle") else {
            return;
        };
        let plain = c9s::decrypt(&bundle).unwrap();
        let rgb = texture::texture_from_bundle(&unityfs::decode(&plain).unwrap())
            .unwrap()
            .into_rgb()
            .unwrap();
        assert_eq!(rgb.dimensions(), (2048, 1152));
        assert_eq!(
            format!("{:x}", Sha256::digest(rgb.as_raw())),
            "a82dfaec62534e5356d06f95531972980711761693bdc3786a383c157e007002"
        );
        let art = read(&plain).unwrap();
        assert_eq!((art.sprites.width, art.sprites.height), (2048, 1152));
        assert!(!art.sprites.sprites.is_empty());
    }
}
