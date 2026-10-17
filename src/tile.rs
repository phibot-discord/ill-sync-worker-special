//! The prefab the passcode puts on the Phase 1 map in place of the locked tile (`c9s.zFzUbVcZ`): read with
//! phi-unpackd's map tile reader, as the tiles on the maps themselves are

use phi_unpackd::collection_art::{self, TILE_SCRIPT};
pub use phi_unpackd::collection_art::{tile_file, Tile};
use phi_unpackd::texture::Texture;
use phi_unpackd::unity::Assets;
use phi_unpackd::unityfs::Node;

/// The decrypted prefab bundle's one tile: the entry it opens, its label and its picture
pub fn read_tile(nodes: &[Node]) -> Result<(Tile, Texture), String> {
    let assets = Assets::new(nodes)?;
    match assets.behaviours(TILE_SCRIPT).as_slice() {
        [one] => collection_art::read_tile_at(&assets, *one),
        found => Err(format!("{} map tiles in the bundle", found.len())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixture;
    use phi_unpackd::{texture, unityfs};
    use sha2::{Digest, Sha256};

    #[test]
    fn reads_the_passcode_tile_of_the_401_apk() {
        let Some(bundle) = fixture("c9s/tile.bundle") else {
            return;
        };
        let nodes = unityfs::decode(&crate::c9s::decrypt(&bundle).unwrap()).unwrap();
        let (tile, tex) = read_tile(&nodes).unwrap();
        assert_eq!(
            tile,
            Tile {
                id: "blackhole0".into(),
                width: 508,
                height: 456,
                label: "答案".into()
            }
        );
        // UnityPy's RGBA image of the sprite's texture, not the larger locked tile's (509x456) in the same bundle
        assert_eq!(tex.color, image::ColorType::Rgba8);
        assert_eq!(
            format!("{:x}", Sha256::digest(&tex.pixels)),
            "121b1be57f979dc11a900a37d825fd73cab759f9081058cc510fe5d8b736f49a"
        );
        let png = texture::encode_png(&tex).unwrap();
        let back = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(back.as_raw(), &tex.pixels);
    }
}
