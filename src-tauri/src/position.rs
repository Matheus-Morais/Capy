use crate::geometry::Point;
use std::{fs, io, path::Path};

pub fn load(path: &Path) -> Option<Point> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > 1024 {
        return None;
    }
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}
pub fn save(path: &Path, point: Point) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec(&point)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn position_round_trip_and_invalid_json_fallback() {
        let path =
            std::env::temp_dir().join(format!("capy-position-test-{}.json", std::process::id()));
        let point = Point { x: -640, y: 384 };
        save(&path, point).unwrap();
        assert_eq!(load(&path), Some(point));
        fs::write(&path, b"{broken").unwrap();
        assert_eq!(load(&path), None);
        fs::remove_file(path).unwrap();
    }
}
