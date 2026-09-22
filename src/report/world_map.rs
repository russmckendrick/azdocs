//! The resource-locations world map for the printed assessment and the
//! draw.io workbook: the desktop Overview's map (same land outline,
//! projection and, in print, crop) redrawn in the theme's palette. Print has
//! no hover, so marker area carries the resource count instead; the regions
//! chart beneath names every location, and the workbook labels each marker.

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

/// Which latitudes a map shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    /// Cropped to where Azure regions are, as on the desktop Overview: no
    /// region is south of Chile or north of Norway. The print figures.
    Print,
    /// The whole globe, pole to pole: the editable draw.io sheet, where a
    /// crop reads as a cut-off rather than a tight figure.
    Globe,
}

impl View {
    fn latitudes(self) -> (f64, f64) {
        match self {
            Self::Print => (74.0, -56.0),
            Self::Globe => (90.0, -90.0),
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Print => 0,
            Self::Globe => 1,
        }
    }
}

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

fn frame_for(view: View) -> &'static Frame {
    static FRAMES: [OnceLock<Frame>; 2] = [OnceLock::new(), OnceLock::new()];
    FRAMES[view.index()].get_or_init(|| {
        let (top, bottom) = view.latitudes();
        let (x_min, _) = natural_earth(-180.0, 0.0);
        let (x_max, _) = natural_earth(180.0, 0.0);
        let (_, y_top) = natural_earth(0.0, top);
        let (_, y_bottom) = natural_earth(0.0, bottom);
        let scale = MAP_WIDTH / (x_max - x_min);
        Frame {
            x_min,
            y_top,
            scale,
            height: ((y_bottom - y_top) * scale).round(),
        }
    })
}

#[cfg(test)]
fn project(longitude: f64, latitude: f64) -> (f64, f64) {
    project_in(View::Print, longitude, latitude)
}

fn project_in(view: View, longitude: f64, latitude: f64) -> (f64, f64) {
    let f = frame_for(view);
    let (x, y) = natural_earth(longitude, latitude);
    ((x - f.x_min) * f.scale, (y - f.y_top) * f.scale)
}

fn land_path(view: View) -> &'static str {
    static PATHS: [OnceLock<String>; 2] = [OnceLock::new(), OnceLock::new()];
    PATHS[view.index()].get_or_init(|| {
        let rings: Vec<Vec<f64>> = serde_json::from_str(LAND_OUTLINE).unwrap_or_else(|err| {
            panic!("embedded land outline is valid; guaranteed by unit test: {err}")
        });
        let mut path = String::new();
        for ring in rings {
            for (index, [lon, lat]) in ring.as_chunks::<2>().0.iter().enumerate() {
                let (x, y) = project_in(view, *lon, *lat);
                let command = if index == 0 { 'M' } else { 'L' };
                path.push_str(&format!("{command}{x:.1},{y:.1}"));
            }
            path.push('Z');
        }
        path
    })
}

fn graticule_path(view: View) -> String {
    let (top, bottom) = view.latitudes();
    let mut path = String::new();
    for longitude in (-180..=180).step_by(30) {
        let mut latitude = bottom;
        let mut first = true;
        while latitude <= top {
            let (x, y) = project_in(view, f64::from(longitude), latitude);
            path.push_str(&format!("{}{x:.1},{y:.1}", if first { 'M' } else { 'L' }));
            first = false;
            latitude += 2.0;
        }
    }
    for latitude in [-60.0, -30.0, 0.0, 30.0, 60.0] {
        if latitude <= bottom || latitude >= top {
            continue;
        }
        let (left, y) = project_in(view, -180.0, latitude);
        let (right, _) = project_in(view, 180.0, latitude);
        path.push_str(&format!("M{left:.1},{y:.1}H{right:.1}"));
    }
    path
}

/// The globe's outline within the view's latitudes, filled as sea.
fn sea_path(view: View) -> String {
    let (top, bottom) = view.latitudes();
    let mut points = Vec::new();
    let mut latitude = bottom;
    while latitude <= top {
        points.push(project_in(view, -180.0, latitude));
        latitude += 2.0;
    }
    let mut latitude = top;
    while latitude >= bottom {
        points.push(project_in(view, 180.0, latitude));
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
    /// Index into the caller's locations, so a label can name the marker.
    location: usize,
}

/// Nudges overlapping markers apart (the desktop's `spreadMarkers`, with the
/// gap taken from each pair's radii). Deterministic: relaxed in input order,
/// coincident markers split on a fixed angle. Each keeps its true anchor.
fn spread(view: View, markers: &mut [Marker]) {
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
    let height = frame_for(view).height;
    for marker in markers {
        marker.x = marker.x.clamp(marker.radius, MAP_WIDTH - marker.radius);
        marker.y = marker.y.clamp(marker.radius, height - marker.radius);
    }
}

#[cfg(test)]
fn markers(locations: &[NameCount]) -> Vec<Marker> {
    markers_in(View::Print, locations)
}

fn markers_in(view: View, locations: &[NameCount]) -> Vec<Marker> {
    let regions = region_catalogue();
    let placed: Vec<_> = locations
        .iter()
        .enumerate()
        .filter_map(|(index, l)| {
            regions
                .get(&l.name.to_ascii_lowercase())
                .map(|r| (r, l.count, index))
        })
        .collect();
    let max = placed.iter().map(|(_, count, _)| *count).max().unwrap_or(0);
    let mut markers: Vec<_> = placed
        .into_iter()
        .map(|(region, count, location)| {
            let (x, y) = project_in(view, region.longitude, region.latitude);
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
                location,
            }
        })
        .collect();
    spread(view, &mut markers);
    markers
}

fn attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

/// A marker as drawn, in the figure's pixels, for a consumer that labels it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacedMarker {
    /// Index into the locations the figure was drawn from.
    pub location: usize,
    pub x: f64,
    pub y: f64,
    /// Including the halo, so a label clears everything drawn.
    pub radius: f64,
}

/// A rendered map with its pixel size and where each marker landed.
pub(crate) struct MapFigure {
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub markers: Vec<PlacedMarker>,
}

/// How far a marker's halo extends past the marker, in map units.
const RING_WIDTH: f64 = 2.2;

/// The map as a standalone SVG, or `None` when no location has coordinates
/// (e.g. an estate of global resources only), so there is nothing to place.
pub(crate) fn render(locations: &[NameCount], palette: &Palette) -> Option<String> {
    figure(locations, palette, View::Print, PIXEL_WIDTH).map(|figure| figure.svg)
}

/// The map at `view`'s latitudes, `pixel_width` wide, with its markers.
pub(crate) fn figure(
    locations: &[NameCount],
    palette: &Palette,
    view: View,
    pixel_width: f64,
) -> Option<MapFigure> {
    let markers = markers_in(view, locations);
    if markers.is_empty() {
        return None;
    }
    let height = frame_for(view).height;
    let pixel_height = (height * pixel_width / MAP_WIDTH).round();
    let scale = pixel_width / MAP_WIDTH;
    let map = &palette.map;
    let (sea, land, land_south, coast, grid, marker, leader, surface) = (
        attr(&map.sea),
        attr(&map.land),
        attr(&map.land_south),
        attr(&map.coast),
        attr(&map.grid),
        attr(&map.marker),
        attr(&map.leader),
        attr(&palette.surface),
    );
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pixel_width}\" height=\"{pixel_height}\" viewBox=\"0 0 {MAP_WIDTH} {height}\">\
         <defs><linearGradient id=\"land\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\">\
         <stop offset=\"0\" stop-color=\"{land}\"/><stop offset=\"1\" stop-color=\"{land_south}\"/>\
         </linearGradient></defs>\
         <path d=\"{}\" fill=\"{sea}\"/>\
         <path d=\"{}\" fill=\"none\" stroke=\"{grid}\" stroke-width=\"0.4\"/>\
         <path d=\"{}\" fill=\"url(#land)\" fill-rule=\"evenodd\" stroke=\"{coast}\" stroke-width=\"0.3\" stroke-linejoin=\"round\"/>",
        sea_path(view),
        graticule_path(view),
        land_path(view),
    );
    for m in &markers {
        if (m.x, m.y) != (m.anchor_x, m.anchor_y) {
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{leader}\" stroke-width=\"0.5\"/>\
                 <circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"1\" fill=\"{leader}\"/>",
                m.anchor_x, m.anchor_y, m.x, m.y, m.anchor_x, m.anchor_y,
            ));
        }
    }
    // The desktop's halo: a faint ring of the marker colour around each dot.
    if !map.marker_ring.is_empty() {
        let ring = attr(&map.marker_ring);
        for m in &markers {
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{ring}\" fill-opacity=\"0.12\" stroke=\"{ring}\" stroke-opacity=\"0.4\" stroke-width=\"0.5\"/>",
                m.x,
                m.y,
                m.radius + RING_WIDTH,
            ));
        }
    }
    // Busiest first, so smaller markers are painted over larger neighbours.
    for m in &markers {
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{marker}\" fill-opacity=\"0.85\" stroke=\"{surface}\" stroke-width=\"0.8\"/>",
            m.x, m.y, m.radius,
        ));
    }
    svg.push_str("</svg>");
    let ring = if map.marker_ring.is_empty() {
        0.0
    } else {
        RING_WIDTH
    };
    Some(MapFigure {
        svg,
        width: pixel_width,
        height: pixel_height,
        markers: markers
            .iter()
            .map(|m| PlacedMarker {
                location: m.location,
                x: m.x * scale,
                y: m.y * scale,
                radius: (m.radius + ring) * scale,
            })
            .collect(),
    })
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
