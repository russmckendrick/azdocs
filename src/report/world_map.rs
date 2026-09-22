//! The resource-locations world map for the printed assessment: the desktop
//! Overview's map (same land outline, projection and crop) redrawn in the
//! theme's print palette. Print has no hover, so marker area carries the
//! resource count instead; the regions chart beneath names every location.

use std::sync::OnceLock;

use super::NameCount;
use super::theme::Palette;
use crate::model::azure_values::region_catalogue;

/// Written by `desktop/scripts/build-land-outline.mjs` alongside the
/// desktop's copy; `world-map.test.ts` checks the two agree.
const LAND_OUTLINE: &str = include_str!("../../data/land_outline.json");

const RADIANS: f64 = std::f64::consts::PI / 180.0;
/// Map units, as in the desktop: one unit is about one pixel in a panel.
const MAP_WIDTH: f64 = 400.0;
/// Rendered width; the PDF and DOCX scale it to the text column either way,
/// this only sets the DOCX raster's resolution.
const PIXEL_WIDTH: f64 = 680.0;
// No Azure region is south of Chile or north of Norway.
const LATITUDE_TOP: f64 = 74.0;
const LATITUDE_BOTTOM: f64 = -56.0;
const MIN_RADIUS: f64 = 2.5;
const MAX_RADIUS: f64 = 9.0;
/// Clear space kept between neighbouring markers after spreading.
const MARKER_GAP: f64 = 1.5;

/// Natural Earth I, the projection `desktop/src/components/world-map.ts`
/// uses, so a region lands on the same coastline in both surfaces.
fn natural_earth(longitude: f64, latitude: f64) -> (f64, f64) {
    let lambda = longitude * RADIANS;
    let phi = latitude * RADIANS;
    let phi2 = phi * phi;
    let phi4 = phi2 * phi2;
    let x = lambda
        * (0.8707 - 0.131979 * phi2
            + phi4 * (-0.013791 + phi4 * (0.003971 * phi2 - 0.001529 * phi4)));
    let y = phi
        * (1.007226 + phi2 * (0.015085 + phi4 * (-0.044475 + 0.028874 * phi2 - 0.005916 * phi4)));
    (x, -y)
}

struct Frame {
    x_min: f64,
    y_top: f64,
    scale: f64,
    height: f64,
}

fn frame() -> &'static Frame {
    static FRAME: OnceLock<Frame> = OnceLock::new();
    FRAME.get_or_init(|| {
        let (x_min, _) = natural_earth(-180.0, 0.0);
        let (x_max, _) = natural_earth(180.0, 0.0);
        let (_, y_top) = natural_earth(0.0, LATITUDE_TOP);
        let (_, y_bottom) = natural_earth(0.0, LATITUDE_BOTTOM);
        let scale = MAP_WIDTH / (x_max - x_min);
        Frame {
            x_min,
            y_top,
            scale,
            height: ((y_bottom - y_top) * scale).round(),
        }
    })
}

fn project(longitude: f64, latitude: f64) -> (f64, f64) {
    let f = frame();
    let (x, y) = natural_earth(longitude, latitude);
    ((x - f.x_min) * f.scale, (y - f.y_top) * f.scale)
}

fn land_path() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        let rings: Vec<Vec<f64>> = serde_json::from_str(LAND_OUTLINE).unwrap_or_else(|err| {
            panic!("embedded land outline is valid; guaranteed by unit test: {err}")
        });
        let mut path = String::new();
        for ring in rings {
            for (index, point) in ring.chunks_exact(2).enumerate() {
                let (x, y) = project(point[0], point[1]);
                let command = if index == 0 { 'M' } else { 'L' };
                path.push_str(&format!("{command}{x:.1},{y:.1}"));
            }
            path.push('Z');
        }
        path
    })
}

fn graticule_path() -> String {
    let mut path = String::new();
    for longitude in (-180..=180).step_by(30) {
        let mut latitude = LATITUDE_BOTTOM;
        let mut first = true;
        while latitude <= LATITUDE_TOP {
            let (x, y) = project(f64::from(longitude), latitude);
            path.push_str(&format!("{}{x:.1},{y:.1}", if first { 'M' } else { 'L' }));
            first = false;
            latitude += 2.0;
        }
    }
    for latitude in [-30.0, 0.0, 30.0, 60.0] {
        let (left, y) = project(-180.0, latitude);
        let (right, _) = project(180.0, latitude);
        path.push_str(&format!("M{left:.1},{y:.1}H{right:.1}"));
    }
    path
}

/// The globe's outline within the cropped latitudes, filled as sea.
fn sea_path() -> String {
    let mut points = Vec::new();
    let mut latitude = LATITUDE_BOTTOM;
    while latitude <= LATITUDE_TOP {
        points.push(project(-180.0, latitude));
        latitude += 2.0;
    }
    let mut latitude = LATITUDE_TOP;
    while latitude >= LATITUDE_BOTTOM {
        points.push(project(180.0, latitude));
        latitude -= 2.0;
    }
    let mut path = String::new();
    for (index, (x, y)) in points.iter().enumerate() {
        path.push_str(&format!(
            "{}{x:.1},{y:.1}",
            if index == 0 { 'M' } else { 'L' }
        ));
    }
    path.push('Z');
    path
}

#[derive(Debug, Clone, PartialEq)]
struct Marker {
    x: f64,
    y: f64,
    anchor_x: f64,
    anchor_y: f64,
    radius: f64,
}

/// Nudges overlapping markers apart (the desktop's `spreadMarkers`, with the
/// gap taken from each pair's radii). Deterministic: relaxed in input order,
/// coincident markers split on a fixed angle. Each keeps its true anchor.
fn spread(markers: &mut [Marker]) {
    for _ in 0..80 {
        let mut moved = false;
        for a in 0..markers.len() {
            for b in a + 1..markers.len() {
                let gap = markers[a].radius + markers[b].radius + MARKER_GAP;
                let mut dx = markers[b].x - markers[a].x;
                let mut dy = markers[b].y - markers[a].y;
                let mut distance = dx.hypot(dy);
                if distance >= gap {
                    continue;
                }
                if distance < 0.01 {
                    let angle = (b as f64 * 2.399_963) % std::f64::consts::TAU;
                    dx = angle.cos();
                    dy = angle.sin();
                    distance = 1.0;
                }
                let push = (gap - distance) / 2.0 + 0.01;
                let (ux, uy) = (dx / distance, dy / distance);
                markers[a].x -= ux * push;
                markers[a].y -= uy * push;
                markers[b].x += ux * push;
                markers[b].y += uy * push;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    let height = frame().height;
    for marker in markers {
        marker.x = marker.x.clamp(marker.radius, MAP_WIDTH - marker.radius);
        marker.y = marker.y.clamp(marker.radius, height - marker.radius);
    }
}

fn markers(locations: &[NameCount]) -> Vec<Marker> {
    let regions = region_catalogue();
    let placed: Vec<_> = locations
        .iter()
        .filter_map(|l| {
            regions
                .get(&l.name.to_ascii_lowercase())
                .map(|r| (r, l.count))
        })
        .collect();
    let max = placed.iter().map(|(_, count)| *count).max().unwrap_or(0);
    let mut markers: Vec<_> = placed
        .into_iter()
        .map(|(region, count)| {
            let (x, y) = project(region.longitude, region.latitude);
            // Area, not radius, follows the count.
            let share = if max == 0 {
                0.0
            } else {
                (count as f64 / max as f64).sqrt()
            };
            Marker {
                x,
                y,
                anchor_x: x,
                anchor_y: y,
                radius: MIN_RADIUS + (MAX_RADIUS - MIN_RADIUS) * share,
            }
        })
        .collect();
    spread(&mut markers);
    markers
}

fn attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

/// The map as a standalone SVG, or `None` when no location has coordinates
/// (e.g. an estate of global resources only), so there is nothing to place.
pub(crate) fn render(locations: &[NameCount], palette: &Palette) -> Option<String> {
    let markers = markers(locations);
    if markers.is_empty() {
        return None;
    }
    let height = frame().height;
    let pixel_height = (height * PIXEL_WIDTH / MAP_WIDTH).round();
    let (sea, land, rule, surface, accent) = (
        attr(&palette.zebra),
        attr(&palette.rule),
        attr(&palette.muted),
        attr(&palette.surface),
        attr(&palette.accent),
    );
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{PIXEL_WIDTH}\" height=\"{pixel_height}\" viewBox=\"0 0 {MAP_WIDTH} {height}\">\
         <path d=\"{}\" fill=\"{sea}\"/>\
         <path d=\"{}\" fill=\"none\" stroke=\"{land}\" stroke-width=\"0.4\"/>\
         <path d=\"{}\" fill=\"{land}\" fill-rule=\"evenodd\" stroke=\"{surface}\" stroke-width=\"0.3\" stroke-linejoin=\"round\"/>",
        sea_path(),
        graticule_path(),
        land_path(),
    );
    for m in &markers {
        if (m.x, m.y) != (m.anchor_x, m.anchor_y) {
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{rule}\" stroke-width=\"0.5\"/>\
                 <circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"1\" fill=\"{rule}\"/>",
                m.anchor_x, m.anchor_y, m.x, m.y, m.anchor_x, m.anchor_y,
            ));
        }
    }
    // Busiest first, so smaller markers are painted over larger neighbours.
    for m in &markers {
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{accent}\" fill-opacity=\"0.85\" stroke=\"{surface}\" stroke-width=\"0.8\"/>",
            m.x, m.y, m.radius,
        ));
    }
    svg.push_str("</svg>");
    Some(svg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(name: &str, count: usize) -> NameCount {
        NameCount {
            name: name.to_owned(),
            display: name.to_owned(),
            count,
        }
    }

    #[test]
    fn land_outline_parses_into_even_length_rings() {
        let rings: Vec<Vec<f64>> = serde_json::from_str(LAND_OUTLINE).expect("valid JSON");
        assert!(rings.len() > 100);
        assert!(rings.iter().all(|r| r.len() >= 8 && r.len() % 2 == 0));
    }

    #[test]
    fn render_returns_none_when_no_location_has_coordinates() {
        let palette = Palette::default();
        assert!(render(&[location("global", 4), location("(none)", 2)], &palette).is_none());
    }

    #[test]
    fn render_draws_one_marker_per_placeable_region() {
        let svg = render(
            &[
                location("uksouth", 10),
                location("global", 3),
                location("eastus", 1),
            ],
            &Palette::default(),
        )
        .expect("two regions have coordinates");
        assert_eq!(svg.matches("fill-opacity=\"0.85\"").count(), 2);
        assert!(usvg::Tree::from_str(&svg, &usvg::Options::default()).is_ok());
    }

    #[test]
    fn markers_scale_area_with_count() {
        let placed = markers(&[location("uksouth", 100), location("eastus", 1)]);
        assert_eq!(placed[0].radius, MAX_RADIUS);
        assert!(placed[1].radius < placed[0].radius);
    }

    #[test]
    fn spread_separates_neighbouring_regions_and_keeps_anchors() {
        let placed = markers(&[
            location("uksouth", 5),
            location("ukwest", 5),
            location("westeurope", 5),
            location("northeurope", 5),
        ]);
        for (a, first) in placed.iter().enumerate() {
            for second in &placed[a + 1..] {
                let distance = (first.x - second.x).hypot(first.y - second.y);
                assert!(distance >= first.radius + second.radius + MARKER_GAP - 0.1);
            }
        }
        let (x, y) = {
            let r = &region_catalogue()["uksouth"];
            project(r.longitude, r.latitude)
        };
        assert_eq!((placed[0].anchor_x, placed[0].anchor_y), (x, y));
    }

    #[test]
    fn render_is_deterministic() {
        let locations = [location("uksouth", 3), location("ukwest", 3)];
        let palette = Palette::default();
        assert_eq!(render(&locations, &palette), render(&locations, &palette));
    }
}
