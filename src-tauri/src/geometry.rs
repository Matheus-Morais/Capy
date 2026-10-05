use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Area {
    pub fn contains(self, point: Point) -> bool {
        let (x, y) = (i64::from(point.x), i64::from(point.y));
        x >= i64::from(self.x)
            && y >= i64::from(self.y)
            && x < i64::from(self.x) + i64::from(self.width)
            && y < i64::from(self.y) + i64::from(self.height)
    }
    pub fn clamp(self, point: Point, width: u32, height: u32) -> Point {
        let max_x = (i64::from(self.x) + i64::from(self.width.saturating_sub(width)))
            .min(i64::from(i32::MAX));
        let max_y = (i64::from(self.y) + i64::from(self.height.saturating_sub(height)))
            .min(i64::from(i32::MAX));
        Point {
            x: i64::from(point.x).clamp(i64::from(self.x), max_x) as i32,
            y: i64::from(point.y).clamp(i64::from(self.y), max_y) as i32,
        }
    }
    pub fn bottom_right(self, width: u32, height: u32) -> Point {
        let corner = self.clamp(
            Point {
                x: i32::MAX,
                y: i32::MAX,
            },
            width,
            height,
        );
        self.clamp(
            Point {
                x: corner.x.saturating_sub(24),
                y: corner.y.saturating_sub(24),
            },
            width,
            height,
        )
    }
}

pub fn restored_position(
    saved: Option<Point>,
    areas: &[Area],
    primary: Area,
    width: u32,
    height: u32,
) -> Point {
    if let Some(point) = saved {
        if let Some(area) = areas.iter().find(|a| a.contains(point)) {
            return area.clamp(point, width, height);
        }
    }
    primary.bottom_right(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamp_with_negative_monitor_origin() {
        let area = Area {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1040,
        };
        assert_eq!(
            area.clamp(Point { x: -5, y: 999 }, 200, 180),
            Point { x: -200, y: 860 }
        );
        assert_eq!(
            area.clamp(Point { x: -4000, y: -100 }, 200, 180),
            Point { x: -1920, y: 0 }
        );
    }
    #[test]
    fn stale_monitor_position_recovers_inside_primary() {
        let primary = Area {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        assert_eq!(
            restored_position(
                Some(Point { x: -9000, y: 50 }),
                &[primary],
                primary,
                200,
                180
            ),
            Point { x: 1696, y: 836 }
        );
    }
    #[test]
    fn extreme_values_do_not_overflow() {
        let area = Area {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        assert_eq!(
            area.clamp(
                Point {
                    x: i32::MAX,
                    y: i32::MIN
                },
                200,
                180
            ),
            Point { x: 1720, y: 0 }
        );
    }
}
