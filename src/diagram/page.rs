//! Page budget and density rungs: how much diagram fits on a sheet of paper
//! and still reads.
//!
//! Every export path eventually puts a diagram on A4 — the PDF places the SVG
//! directly, the DOCX rasterises it to the text width, the HTML site scales it
//! to the content column. Sizing used to be left to the emitters, so a resource
//! group with 59 members produced a 3382px-wide picture whose labels landed at
//! 1.6pt once scaled to the page. This module is the single place that decides
//! the shape, so all of them agree.

/// One layout pixel is exactly 0.25 mm. A binary-exact fraction, so an integer
/// pixel is always an exact millimetre quantity and no float drift reaches the
/// viewBox.
pub const PX_PER_MM: f64 = 4.0;

const MM_PER_POINT: f64 = 25.4 / 72.0;

/// How much of the estate a diagram is asked to show.
///
/// The two consumers want opposite things. A report has to fit A4 portrait, so
/// it takes a summary: resources aggregated by type, canvas snapped to a share
/// of the page. A standalone export has no page to respect, so it takes the
/// lot — every resource drawn, on whatever canvas the content needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramDetail {
    /// Embedded in a document. Aggregated, page-fraction canvas.
    #[default]
    Summary,
    /// Exported on its own. Every resource, natural canvas.
    Full,
}

/// Working width for a full-detail export. Roughly the proportions of the
/// reference Azure topologies, which are landscape and need room for a VNet
/// column beside a wide grid of resources.
const FULL_WIDTH: f64 = 1400.0;

impl DiagramDetail {
    /// Width the layout lays out into.
    pub fn canvas_width(self) -> f64 {
        match self {
            Self::Summary => A4_PORTRAIT.width,
            Self::Full => FULL_WIDTH,
        }
    }

    /// Whether resources collapse into per-type tiles with a count.
    pub fn aggregates(self) -> bool {
        self == Self::Summary
    }

    /// Whether the canvas is rounded to a share of the page.
    pub fn snaps_to_page(self) -> bool {
        self == Self::Summary
    }
}

/// The share of an A4 portrait page a diagram occupies.
///
/// Every diagram is emitted at exactly one of these sizes rather than at
/// whatever its content happens to measure. A report then stacks predictable
/// blocks — two halves or three thirds to a page — instead of a run of
/// arbitrary rectangles that each downscale by a different amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageFraction {
    Quarter,
    Third,
    Half,
    Full,
}

impl PageFraction {
    /// Smallest to largest, which is the order the fit search wants.
    pub const ALL: [Self; 4] = [Self::Quarter, Self::Third, Self::Half, Self::Full];

    pub fn divisor(self) -> f64 {
        match self {
            Self::Quarter => 4.0,
            Self::Third => 3.0,
            Self::Half => 2.0,
            Self::Full => 1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Quarter => "quarter page",
            Self::Third => "third page",
            Self::Half => "half page",
            Self::Full => "full page",
        }
    }

    /// Canvas in layout pixels: always the full content width, and the page
    /// height divided by this fraction.
    pub fn canvas(self) -> (f64, f64) {
        (
            A4_PORTRAIT.width,
            (A4_PORTRAIT.height / self.divisor()).round(),
        )
    }

    /// Smallest fraction whose canvas holds `width` x `height` of content
    /// without shrinking it below `min_scale`. Falls back to a full page.
    pub fn fit(width: f64, height: f64, min_scale: f64) -> Self {
        Self::ALL
            .into_iter()
            .find(|fraction| {
                let (canvas_width, canvas_height) = fraction.canvas();
                if width <= 0.0 || height <= 0.0 {
                    return true;
                }
                (canvas_width / width).min(canvas_height / height) >= min_scale
            })
            .unwrap_or(Self::Full)
    }
}

/// Printable area for a target sheet, in layout pixels.
#[derive(Debug, Clone, Copy)]
pub struct PageBudget {
    pub width: f64,
    pub height: f64,
}

/// A4 portrait at the shipped 2cm margin gives a 170 x 257mm body; 7mm is
/// reserved for the figure caption and border inset.
pub const A4_PORTRAIT: PageBudget = PageBudget {
    width: 170.0 * PX_PER_MM,
    height: 250.0 * PX_PER_MM,
};

impl PageBudget {
    /// Scale applied when a `width` x `height` diagram is fitted to this page.
    /// Capped at 1.0 — a small diagram is printed at true size, never blown up.
    pub fn scale(&self, width: f64, height: f64) -> f64 {
        if width <= 0.0 || height <= 0.0 {
            return 1.0;
        }
        (self.width / width).min(self.height / height).min(1.0)
    }

    /// On-paper size of a `font_px` label once the diagram is fitted.
    pub fn label_pt(&self, font_px: f64, width: f64, height: f64) -> f64 {
        font_px * self.scale(width, height) / PX_PER_MM / MM_PER_POINT
    }
}

/// Geometry for one density level. Chosen once per graph from its box count,
/// so a resource type is never drawn at two sizes within one picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rung {
    pub name: &'static str,
    pub leaf_width: f64,
    pub leaf_height: f64,
    pub icon: f64,
    pub label_px: f64,
    pub container_label_px: f64,
    pub wrap_chars: usize,
    pub wrap_lines: usize,
    /// Childless containers — peering stubs — use this instead of a leaf slot.
    pub pill_width: f64,
    pub pill_height: f64,
    pub padding: f64,
    pub gutter: f64,
    pub title_band: f64,
}

/// Comfortable: the long tail of small resource groups.
pub const COMFORTABLE: Rung = Rung {
    name: "comfortable",
    leaf_width: 152.0,
    leaf_height: 112.0,
    icon: 48.0,
    label_px: 11.0,
    container_label_px: 13.0,
    wrap_chars: 15,
    wrap_lines: 2,
    pill_width: 176.0,
    pill_height: 96.0,
    padding: 20.0,
    gutter: 24.0,
    title_band: 28.0,
};

pub const COMPACT: Rung = Rung {
    name: "compact",
    leaf_width: 116.0,
    leaf_height: 88.0,
    icon: 40.0,
    label_px: 10.0,
    container_label_px: 12.0,
    wrap_chars: 12,
    wrap_lines: 2,
    pill_width: 140.0,
    pill_height: 80.0,
    padding: 14.0,
    gutter: 18.0,
    title_band: 24.0,
};

pub const DENSE: Rung = Rung {
    name: "dense",
    leaf_width: 88.0,
    leaf_height: 64.0,
    icon: 32.0,
    label_px: 9.0,
    container_label_px: 11.0,
    wrap_chars: 12,
    wrap_lines: 1,
    pill_width: 112.0,
    pill_height: 64.0,
    padding: 10.0,
    gutter: 12.0,
    title_band: 20.0,
};

/// Boundaries sit in the empty gaps of a real estate's resource-group size
/// distribution, so a group does not flip rung on a single added resource.
const COMFORTABLE_MAX: usize = 14;
const COMPACT_MAX: usize = 32;

/// Pick the rung for a graph with `boxes` drawable leaves and stubs.
pub fn rung_for(boxes: usize) -> Rung {
    if boxes <= COMFORTABLE_MAX {
        COMFORTABLE
    } else if boxes <= COMPACT_MAX {
        COMPACT
    } else {
        DENSE
    }
}

impl Rung {
    /// Metrics shrink with nesting depth: an RG > VNet > Subnet > leaf shape
    /// otherwise pays padding four times over. Fonts and leaf sizes never
    /// decay — only the chrome around them.
    pub fn at_depth(&self, depth: usize) -> Self {
        let shrink = 2.0 * depth as f64;
        Self {
            padding: (self.padding - shrink).max(6.0),
            gutter: (self.gutter - shrink).max(8.0),
            title_band: (self.title_band - shrink).max(16.0),
            ..*self
        }
    }

    /// Outer size of a `count`-item grid in `columns`, without placing anything.
    pub fn grid_extent(&self, count: usize, columns: usize) -> (f64, f64) {
        let columns = columns.max(1);
        let spanned = count.min(columns);
        let rows = count.div_ceil(columns);
        let width = spanned as f64 * self.leaf_width
            + spanned.saturating_sub(1) as f64 * self.gutter
            + 2.0 * self.padding;
        let height = self.title_band
            + rows as f64 * self.leaf_height
            + rows.saturating_sub(1) as f64 * self.gutter
            + 2.0 * self.padding;
        (width, height)
    }

    /// Columns that keep a `count`-item grid inside `budget` and as close to
    /// the page's own proportions as the content allows.
    ///
    /// The old `sqrt(count * 1.6)` deliberately spread children *wide*, which
    /// is the opposite of what a portrait sheet needs; it is what left every
    /// diagram in a real estate unreadable once scaled to the page.
    pub fn columns(&self, count: usize, budget: &PageBudget) -> usize {
        if count <= 1 {
            return 1;
        }
        let target = budget.width / budget.height;
        let ceiling = self.max_columns(budget);
        (1..=ceiling.max(1))
            .map(|columns| {
                let (width, height) = self.grid_extent(count, columns);
                let rows = count.div_ceil(columns);
                // Packing waste first — an unfilled last row reads as a gap —
                // then how far the block sits from the page's proportions.
                let waste = (rows * columns - count) as f64 / count as f64;
                let shape = (width / height / target).ln().abs();
                (waste + 0.35 * shape, columns)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
            .map_or(1, |(_, columns)| columns)
    }

    /// Widest grid that still fits the page without downscaling.
    fn max_columns(&self, budget: &PageBudget) -> usize {
        let usable = budget.width - 2.0 * self.padding;
        let per_column = self.leaf_width + self.gutter;
        (((usable + self.gutter) / per_column).floor() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_rung_steps_down_as_the_box_count_grows() {
        assert_eq!(rung_for(1).name, "comfortable");
        assert_eq!(rung_for(14).name, "comfortable");
        assert_eq!(rung_for(15).name, "compact");
        assert_eq!(rung_for(32).name, "compact");
        assert_eq!(rung_for(33).name, "dense");
        assert_eq!(rung_for(294).name, "dense");
    }

    #[test]
    fn unit_columns_never_overflow_the_page_width() {
        for rung in [COMFORTABLE, COMPACT, DENSE] {
            for count in [2, 6, 13, 28, 42, 59, 294] {
                let columns = rung.columns(count, &A4_PORTRAIT);
                let (width, _) = rung.grid_extent(count, columns);
                assert!(
                    width <= A4_PORTRAIT.width,
                    "{} chose {columns} columns for {count}: {width}px > {}px",
                    rung.name,
                    A4_PORTRAIT.width
                );
            }
        }
    }

    #[test]
    fn unit_columns_prefer_a_wider_grid_when_waste_is_equal() {
        // 4 items pack evenly at 2x2 rather than stacking into one column.
        assert_eq!(COMFORTABLE.columns(4, &A4_PORTRAIT), 2);
    }

    #[test]
    fn unit_depth_decay_shrinks_chrome_but_never_fonts() {
        let deep = COMFORTABLE.at_depth(3);

        assert!(deep.padding < COMFORTABLE.padding);
        assert!(deep.gutter < COMFORTABLE.gutter);
        assert_eq!(deep.label_px, COMFORTABLE.label_px);
        assert_eq!(deep.leaf_width, COMFORTABLE.leaf_width);
    }

    #[test]
    fn unit_depth_decay_stops_at_the_floor() {
        let very_deep = COMFORTABLE.at_depth(50);

        assert_eq!(very_deep.padding, 6.0);
        assert_eq!(very_deep.gutter, 8.0);
        assert_eq!(very_deep.title_band, 16.0);
    }

    #[test]
    fn unit_scale_never_enlarges_a_small_diagram() {
        assert_eq!(A4_PORTRAIT.scale(100.0, 100.0), 1.0);
    }

    /// The whole point of the redesign: a real resource group has to print
    /// above the readability floor.
    #[test]
    fn unit_label_pt_clears_the_floor_for_a_typical_resource_group() {
        let rung = rung_for(13);
        let (width, height) = rung.grid_extent(13, rung.columns(13, &A4_PORTRAIT));

        assert!(
            A4_PORTRAIT.label_pt(rung.label_px, width, height) >= 7.0,
            "13-resource group prints at {}pt",
            A4_PORTRAIT.label_pt(rung.label_px, width, height)
        );
    }
}
