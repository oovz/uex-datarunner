//! Expected OCR output for every bundled test screenshot.
//!
//! Add new fixtures here when new screenshot assets are introduced.
//! `cargo_sizes` is required by the schema; use an empty slice when the
//! model legitimately omits sizes (e.g. because the UI column is cut off).

#[derive(Debug, Clone)]
pub struct ExpectedCommodity {
    pub name: &'static str,
    pub scu: Option<f64>,
    pub price_per_scu: Option<f64>,
    /// Expected cargo box sizes. An empty slice means "sizes not visible".
    pub cargo_sizes: &'static [i64],
    /// UEX inventory-status code (1–7).  This is the language-agnostic value the
    /// UEX `/data_submit` endpoint expects:
    ///   1 = Out of stock / Empty
    ///   2 = Very low
    ///   3 = Low
    ///   4 = Medium
    ///   5 = High
    ///   6 = Very high
    ///   7 = Maximum / Full
    pub status: Option<i64>,
    /// When `true` the commodity may be omitted by the model because part of
    /// the row is cut off in the screenshot.
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct ScreenshotFixture {
    pub filename: &'static str,
    pub market_side: &'static str,
    pub commodities: &'static [ExpectedCommodity],
}

/// Every bundled screenshot and its expected structured output.
pub const ALL_SCREENSHOT_FIXTURES: &[ScreenshotFixture] = &[
    ScreenshotFixture {
        filename: "screenshot-chieng-dark.jpg",
        market_side: "buy",
        commodities: &[
            ExpectedCommodity {
                name: "Scrap",
                scu: Some(2100.0),
                price_per_scu: Some(2990.0),
                cargo_sizes: &[1, 2, 4, 8, 16],
                status: Some(7),
                optional: false,
            },
            ExpectedCommodity {
                name: "Waste",
                scu: Some(1.0),
                price_per_scu: Some(115.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(7),
                optional: false,
            },
            ExpectedCommodity {
                name: "Titanium",
                scu: Some(6000.0),
                price_per_scu: Some(7034.0),
                cargo_sizes: &[8, 16, 24, 32],
                status: Some(7),
                optional: false,
            },
        ],
    },
    ScreenshotFixture {
        filename: "screenshot-arc-l1.jpg",
        market_side: "sell",
        commodities: &[
            ExpectedCommodity {
                name: "Party Favors",
                scu: Some(100.0),
                price_per_scu: Some(4900.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(4),
                optional: false,
            },
            ExpectedCommodity {
                name: "Processed Food",
                scu: Some(0.0),
                price_per_scu: Some(1500.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Construction Materials",
                scu: Some(987.0),
                price_per_scu: Some(12000.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(5),
                optional: false,
            },
            ExpectedCommodity {
                name: "Agricium",
                scu: Some(1008.0),
                price_per_scu: Some(10000.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(6),
                optional: false,
            },
            ExpectedCommodity {
                name: "Nitrogen",
                scu: Some(1973.0),
                price_per_scu: Some(2900.0),
                cargo_sizes: &[], // cargo-size column is cut off in the screenshot
                status: Some(5),
                optional: true,
            },
        ],
    },
    ScreenshotFixture {
        filename: "screenshot-eng-hickes.png",
        market_side: "sell",
        commodities: &[
            ExpectedCommodity {
                name: "Stileron",
                scu: Some(0.0),
                price_per_scu: Some(140000.0),
                cargo_sizes: &[1, 2, 4, 8, 16],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Quartz",
                scu: Some(0.0),
                price_per_scu: Some(4000.0),
                cargo_sizes: &[1, 2, 4, 8, 16],
                status: Some(1),
                optional: false,
            },
        ],
    },
    ScreenshotFixture {
        filename: "screenshot-eng-jackson.jpg",
        market_side: "sell",
        commodities: &[
            ExpectedCommodity {
                name: "Pitambu",
                scu: Some(0.0),
                price_per_scu: Some(63000.0),
                cargo_sizes: &[1, 2, 4, 8, 16],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Prota",
                scu: Some(0.0),
                price_per_scu: Some(68000.0),
                cargo_sizes: &[1, 2, 4, 8, 16],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Stileron",
                scu: Some(0.0),
                price_per_scu: Some(130000.0),
                cargo_sizes: &[1, 2, 4, 8, 16],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Taranite",
                scu: Some(0.0),
                price_per_scu: Some(25000.0),
                cargo_sizes: &[], // cargo-size column is cut off
                status: Some(1),
                optional: true,
            },
        ],
    },
    ScreenshotFixture {
        filename: "screenshot-eng-theta.jpg",
        market_side: "sell",
        commodities: &[
            ExpectedCommodity {
                name: "Processed Food",
                scu: Some(0.0),
                price_per_scu: Some(1500.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Iron",
                scu: Some(0.0),
                price_per_scu: Some(3500.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Audio-Visual Equipment",
                scu: Some(1.0),
                price_per_scu: Some(37000.0),
                cargo_sizes: &[1, 2, 4, 8, 16],
                status: Some(1),
                optional: false,
            },
            ExpectedCommodity {
                name: "Beryl",
                scu: Some(20.0),
                price_per_scu: Some(18000.0),
                cargo_sizes: &[1, 2, 4, 8, 16, 24, 32],
                status: Some(1),
                optional: false,
            },
        ],
    },
];
