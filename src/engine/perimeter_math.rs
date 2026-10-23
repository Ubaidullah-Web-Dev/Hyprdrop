// ==============================================================================
// Hyprdrop Perimeter Math & Morphing Engine
// ==============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScreenEdge {
    Top,
    Right,
    Bottom,
    Left,
    CornerTopRight { len_top: f64, len_right: f64 },
    CornerBottomRight { len_right: f64, len_bottom: f64 },
    CornerBottomLeft { len_bottom: f64, len_left: f64 },
    CornerTopLeft { len_left: f64, len_top: f64 },
}

impl ScreenEdge {
    pub fn name(&self) -> &'static str {
        match self {
            ScreenEdge::Top => "Top Edge",
            ScreenEdge::Right => "Right Edge",
            ScreenEdge::Bottom => "Bottom Edge",
            ScreenEdge::Left => "Left Edge",
            ScreenEdge::CornerTopRight { .. } => "Top-Right Corner",
            ScreenEdge::CornerBottomRight { .. } => "Bottom-Right Corner",
            ScreenEdge::CornerBottomLeft { .. } => "Bottom-Left Corner",
            ScreenEdge::CornerTopLeft { .. } => "Top-Left Corner",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerimeterSpan {
    pub edge: ScreenEdge,
    pub center_s: f64,
    pub length: f64,
    pub start_s: f64,
    pub end_s: f64,
}

impl PerimeterSpan {
    pub fn is_corner(&self) -> bool {
        matches!(
            self.edge,
            ScreenEdge::CornerTopRight { .. }
                | ScreenEdge::CornerBottomRight { .. }
                | ScreenEdge::CornerBottomLeft { .. }
                | ScreenEdge::CornerTopLeft { .. }
        )
    }

    pub fn is_horizontal(&self) -> bool {
        matches!(self.edge, ScreenEdge::Top | ScreenEdge::Bottom)
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self.edge, ScreenEdge::Right | ScreenEdge::Left)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PerimeterMath {
    pub width: f64,
    pub height: f64,
}

impl PerimeterMath {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width: width.max(100.0),
            height: height.max(100.0),
        }
    }

    pub fn total_perimeter(&self) -> f64 {
        2.0 * (self.width + self.height)
    }

    pub fn t_to_s(&self, t: f64) -> f64 {
        let p = self.total_perimeter();
        t.rem_euclid(1.0) * p
    }

    pub fn s_to_t(&self, s: f64) -> f64 {
        let p = self.total_perimeter();
        (s.rem_euclid(p)) / p
    }

    /// Maps cursor (x, y) to the closest point along the screen perimeter s.
    pub fn cursor_to_perimeter_s(&self, cx: f64, cy: f64) -> f64 {
        let w = self.width;
        let h = self.height;

        let dist_top = cy.abs();
        let dist_bottom = (h - cy).abs();
        let dist_left = cx.abs();
        let dist_right = (w - cx).abs();

        let min_dist = dist_top.min(dist_bottom).min(dist_left).min(dist_right);

        if min_dist == dist_top {
            // Top edge: s = x clamped to [0, w]
            cx.clamp(0.0, w)
        } else if min_dist == dist_right {
            // Right edge: s = w + y clamped to [0, h]
            w + cy.clamp(0.0, h)
        } else if min_dist == dist_bottom {
            // Bottom edge: s = w + h + (w - x)
            w + h + (w - cx.clamp(0.0, w))
        } else {
            // Left edge: s = 2w + h + (h - y)
            2.0 * w + h + (h - cy.clamp(0.0, h))
        }
    }

    pub fn edge_name(edge: &ScreenEdge) -> &'static str {
        edge.name()
    }

    pub fn edge_center_coord(&self, span: &PerimeterSpan) -> f64 {
        let c = span.center_s;
        let w = self.width;
        let h = self.height;
        match span.edge {
            ScreenEdge::Top => c,
            ScreenEdge::Right => c - w,
            ScreenEdge::Bottom => w - (c - (w + h)),
            ScreenEdge::Left => h - (c - (2.0 * w + h)),
            _ => 0.0,
        }
    }

    /// Calculates perimeter span and classifies whether it's on an edge or bent across a corner.
    pub fn calculate_span(&self, center_s: f64, length: f64) -> PerimeterSpan {
        let p = self.total_perimeter();
        let half = length / 2.0;

        let c = center_s.rem_euclid(p);
        let s_start = (c - half).rem_euclid(p);
        let s_end = (c + half).rem_euclid(p);

        let c1 = self.width;              // Top-Right
        let c2 = self.width + self.height; // Bottom-Right
        let c3 = 2.0 * self.width + self.height; // Bottom-Left
        let _c0 = 0.0;                     // Top-Left (also p)

        // Check corner intersections
        // 1. Top-Right Corner (s crosses c1)
        if (c - half) < c1 && (c + half) > c1 {
            let len_top = c1 - (c - half);
            let len_right = (c + half) - c1;
            return PerimeterSpan {
                edge: ScreenEdge::CornerTopRight { len_top, len_right },
                center_s: c,
                length,
                start_s: s_start,
                end_s: s_end,
            };
        }

        // 2. Bottom-Right Corner (s crosses c2)
        if (c - half) < c2 && (c + half) > c2 {
            let len_right = c2 - (c - half);
            let len_bottom = (c + half) - c2;
            return PerimeterSpan {
                edge: ScreenEdge::CornerBottomRight { len_right, len_bottom },
                center_s: c,
                length,
                start_s: s_start,
                end_s: s_end,
            };
        }

        // 3. Bottom-Left Corner (s crosses c3)
        if (c - half) < c3 && (c + half) > c3 {
            let len_bottom = c3 - (c - half);
            let len_left = (c + half) - c3;
            return PerimeterSpan {
                edge: ScreenEdge::CornerBottomLeft { len_bottom, len_left },
                center_s: c,
                length,
                start_s: s_start,
                end_s: s_end,
            };
        }

        // 4. Top-Left Corner (s crosses 0 / p)
        if (c - half) < 0.0 || (c + half) > p {
            let (len_left, len_top) = if (c - half) < 0.0 {
                let len_left = half - c;
                let len_top = half + c;
                (len_left, len_top)
            } else {
                let len_left = (c + half) - p;
                let len_top = p - (c - half);
                (len_left, len_top)
            };
            return PerimeterSpan {
                edge: ScreenEdge::CornerTopLeft { len_left, len_top },
                center_s: c,
                length,
                start_s: s_start,
                end_s: s_end,
            };
        }

        // Straight Edges
        let edge = if c < c1 {
            ScreenEdge::Top
        } else if c < c2 {
            ScreenEdge::Right
        } else if c < c3 {
            ScreenEdge::Bottom
        } else {
            ScreenEdge::Left
        };

        PerimeterSpan {
            edge,
            center_s: c,
            length,
            start_s: s_start,
            end_s: s_end,
        }
    }
}
