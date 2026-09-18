//! RFC 0002 §8 geometry predicates shared by abstract-grid and pixel verification.
//! Coordinates use downward-growing `y`.

/// One point of an orthogonal plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// Whether two orthogonal segments meet: parallel ones only where they overlap,
/// perpendicular ones wherever they touch.
fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    let horizontal = a.y == b.y;
    if horizontal == (c.y == d.y) {
        if horizontal {
            a.y == c.y && a.x.min(b.x).max(c.x.min(d.x)) < a.x.max(b.x).min(c.x.max(d.x))
        } else {
            a.x == c.x && a.y.min(b.y).max(c.y.min(d.y)) < a.y.max(b.y).min(c.y.max(d.y))
        }
    } else {
        let (across, down) = if horizontal {
            ((a, b), (c, d))
        } else {
            ((c, d), (a, b))
        };
        down.0.x >= across.0.x.min(across.1.x)
            && down.0.x <= across.0.x.max(across.1.x)
            && across.0.y >= down.0.y.min(down.1.y)
            && across.0.y <= down.0.y.max(down.1.y)
    }
}

/// Whether an orthogonal segment reaches the inside of a rectangle. Running
/// along an edge is not entering it: a cycle boundary is drawn on its own
/// bounds, and its interface meets it there.
#[must_use]
pub fn enters(a: Point, b: Point, (left, top, right, bottom): (i32, i32, i32, i32)) -> bool {
    if a.x == b.x {
        a.x > left && a.x < right && a.y.min(b.y) < bottom && a.y.max(b.y) > top
    } else {
        a.y > top && a.y < bottom && a.x.min(b.x) < right && a.x.max(b.x) > left
    }
}

/// Whether a point lies strictly inside a rectangle.
#[must_use]
pub const fn inside(point: Point, (left, top, right, bottom): (i32, i32, i32, i32)) -> bool {
    point.x > left && point.x < right && point.y > top && point.y < bottom
}

#[must_use]
pub(crate) fn on_segment(point: Point, segment: &[Point]) -> bool {
    point.x >= segment[0].x.min(segment[1].x)
        && point.x <= segment[0].x.max(segment[1].x)
        && point.y >= segment[0].y.min(segment[1].y)
        && point.y <= segment[0].y.max(segment[1].y)
}

/// Whether two routes may meet where they do. RFC 0002 §8 allows a common
/// endpoint and a deliberately shared collinear segment, and only between
/// connections leaving one exit or reaching one destination.
#[must_use]
pub fn compatible(left: &[Point], right: &[Point], shared: bool, meetings: &[Point]) -> bool {
    for a in left.windows(2) {
        for b in right.windows(2) {
            let parallel = (a[0].x == a[1].x) == (b[0].x == b[1].x);
            let allowed = if parallel {
                shared
            } else {
                meetings
                    .iter()
                    .any(|&point| on_segment(point, a) && on_segment(point, b))
            };
            if !allowed && crosses(a[0], a[1], b[0], b[1]) {
                return false;
            }
        }
    }
    true
}

/// A bundle may split or join at a common endpoint or the end of a shared run.
#[must_use]
pub fn bundle_meetings(left: &[Point], right: &[Point]) -> Vec<Point> {
    let mut meetings = Vec::new();
    for (a, b) in [(left.first(), right.first()), (left.last(), right.last())] {
        if a == b {
            meetings.extend(a.copied());
        }
    }
    for a in left.windows(2) {
        for b in right.windows(2) {
            if (a[0].x == a[1].x) == (b[0].x == b[1].x) && crosses(a[0], a[1], b[0], b[1]) {
                meetings.extend(
                    a.iter()
                        .chain(b)
                        .copied()
                        .filter(|&point| on_segment(point, a) && on_segment(point, b)),
                );
            }
        }
    }
    meetings
}

#[must_use]
pub fn overlaps_itself(points: &[Point]) -> bool {
    points.windows(3).any(|points| {
        (points[0].x == points[1].x) == (points[1].x == points[2].x)
            && crosses(points[0], points[1], points[1], points[2])
    }) || points.windows(2).enumerate().any(|(index, segment)| {
        points[index + 2..]
            .windows(2)
            .any(|other| crosses(segment[0], segment[1], other[0], other[1]))
    })
}

/// Drops repeated points and collapses runs of collinear ones, so every bend is
/// a real right angle.
#[must_use]
pub fn straighten(points: Vec<Point>) -> Vec<Point> {
    let mut straight: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        if straight.last() == Some(&point) {
            continue;
        }
        while straight.len() >= 2 {
            let previous = straight[straight.len() - 2];
            let last = straight[straight.len() - 1];
            let collinear = (previous.x == last.x && last.x == point.x)
                || (previous.y == last.y && last.y == point.y);
            if collinear {
                straight.pop();
            } else {
                break;
            }
        }
        straight.push(point);
    }
    straight
}

/// Whether a route turns sideways below its own start and then arrives other
/// than horizontally. An incoming side route must not turn down over the
/// continuation its junction's outgoing connection owns (RFC 0002 §8).
///
/// Returns false for fewer than two points.
#[must_use]
pub fn turns_downward(points: &[Point]) -> bool {
    points
        .windows(2)
        .any(|segment| segment[0].x != segment[1].x && segment[0].y > points[0].y)
        && points[points.len() - 2].y != points[points.len() - 1].y
}
