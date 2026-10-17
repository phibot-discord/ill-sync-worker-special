//! Serialized-field writers for building component bytes in tests

pub fn s(out: &mut Vec<u8>, text: &str) {
    out.extend((text.len() as i32).to_le_bytes());
    out.extend(text.as_bytes());
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

pub fn p(out: &mut Vec<u8>, file: i32, path: i64) {
    out.extend(file.to_le_bytes());
    out.extend(path.to_le_bytes());
}

pub fn header(out: &mut Vec<u8>, game_object: i64, script: (i32, i64)) {
    p(out, 0, game_object);
    out.extend([1, 0, 0, 0]);
    p(out, script.0, script.1);
    s(out, "");
}

/// The 4.0.1 APK's own data (see /tmp/phi-fixtures, written by the fixture script)
pub fn fixture(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("/tmp/phi-fixtures/{name}")).ok()
}
