//! ADR-0013 Phase 7: Visual Glyphs
//!
//! Transform the Fano plane projection of a glyph into visual coordinates
//! for rendering. Each glyph's 7 Fano projection values map to a unique
//! visual form — similar memories produce visually similar glyphs.
//!
//! ## Visual Encoding
//!
//! The 7 Fano plane values map to:
//! - **Shape vertices** (lines 0–2): Triangular shape distortion
//! - **Color** (lines 3–5): RGB color channels
//! - **Texture/density** (line 6): Inner pattern complexity
//!
//! The visual encodes *meaning*, not *privacy*. A sealed memory and an
//! open memory look equally complex as glyphs.

use serde::{Deserialize, Serialize};

use crate::collective::glyph_spec::{Glyph, GlyphSource};
use crate::collective::glyph_store::{GlyphStore, StoredGlyph};
use crate::collective::privacy::PrivacyGlyph;

// ============================================================================
// Visual Coordinate Types
// ============================================================================

/// Visual coordinates derived from a glyph's Fano plane projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphVisual {
    /// Hash of the source glyph
    pub glyph_hash: String,
    /// Triangle vertices (3 points in 2D space)
    pub vertices: [(f64, f64); 3],
    /// RGB color (0.0–1.0 each)
    pub color: (f64, f64, f64),
    /// Inner pattern density (0.0–1.0)
    pub density: f64,
    /// Centroid position (for cluster layout)
    pub centroid: (f64, f64),
    /// Visual size (proportional to amplitude)
    pub size: f64,
}

/// A cluster of visually similar glyphs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphCluster {
    /// Cluster identifier
    pub cluster_id: String,
    /// Centroid of the cluster
    pub centroid: (f64, f64),
    /// Member glyph hashes
    pub members: Vec<String>,
    /// Average color of members
    pub avg_color: (f64, f64, f64),
    /// Number of members
    pub count: usize,
}

// ============================================================================
// Fano → Visual Mapping
// ============================================================================

/// Map a glyph's Fano projection to visual coordinates.
///
/// The 7 Fano values are interpreted as:
/// - `fano[0..3]` → triangle vertex distortions (shape)
/// - `fano[3..6]` → RGB color channels
/// - `fano[6]`    → inner pattern density
pub fn fano_to_visual(glyph: &PrivacyGlyph) -> Option<GlyphVisual> {
    let fano = glyph.fano_projection.as_ref()?;

    // Base equilateral triangle centered at origin with unit radius
    let base_vertices = [
        (0.0, 1.0),                                // top
        (-0.866025, -0.5),                          // bottom-left (cos 210°, sin 210°)
        (0.866025, -0.5),                           // bottom-right (cos 330°, sin 330°)
    ];

    // Distort vertices by Fano values 0–2
    // Each value shifts the vertex radially by ±0.3
    let vertices = [
        distort_vertex(base_vertices[0], fano[0], 0),
        distort_vertex(base_vertices[1], fano[1], 1),
        distort_vertex(base_vertices[2], fano[2], 2),
    ];

    // Color from Fano values 3–5 (normalized to 0–1)
    let color = (
        normalize_fano(fano[3]),
        normalize_fano(fano[4]),
        normalize_fano(fano[5]),
    );

    // Density from Fano value 6
    let density = normalize_fano(fano[6]);

    // Centroid = average of vertices
    let centroid = (
        (vertices[0].0 + vertices[1].0 + vertices[2].0) / 3.0,
        (vertices[0].1 + vertices[1].1 + vertices[2].1) / 3.0,
    );

    // Size proportional to committed amplitude
    let size = glyph.committed_amplitude.abs().min(2.0) / 2.0;

    Some(GlyphVisual {
        glyph_hash: glyph.glyph_hash.clone(),
        vertices,
        color,
        density,
        centroid,
        size,
    })
}

/// Distort a base vertex by a Fano value.
///
/// The distortion pushes the vertex radially outward or inward
/// based on the Fano energy value, with a rotational component
/// keyed to the vertex index.
fn distort_vertex(base: (f64, f64), fano_val: f64, idx: usize) -> (f64, f64) {
    let norm = normalize_fano(fano_val);
    let radial = (norm - 0.5) * 0.6; // ±0.3 radial shift

    // Angle from center to vertex
    let angle = (base.1).atan2(base.0);
    // Add rotational twist based on vertex index
    let twist = idx as f64 * 0.1 * norm;

    (
        base.0 + radial * (angle + twist).cos(),
        base.1 + radial * (angle + twist).sin(),
    )
}

/// Normalize a Fano energy value to [0, 1].
///
/// Fano energies are typically small positive floats. We use a
/// sigmoid-like mapping to compress the range.
fn normalize_fano(val: f64) -> f64 {
    // Sigmoid: 1 / (1 + e^(-k*(x - center)))
    // Tuned for typical Fano energy range [0, 0.5]
    let k = 10.0;
    let center = 0.15;
    1.0 / (1.0 + (-k * (val - center)).exp())
}

// ============================================================================
// SVG Rendering
// ============================================================================

/// Render a glyph visual as an SVG string.
///
/// Produces a standalone SVG element for a single glyph.
pub fn render_svg(visual: &GlyphVisual, canvas_size: f64) -> String {
    let half = canvas_size / 2.0;
    let scale = canvas_size * 0.35;

    // Transform vertices to canvas coordinates
    let pts: Vec<(f64, f64)> = visual.vertices.iter()
        .map(|(x, y)| (half + x * scale, half - y * scale))
        .collect();

    let (r, g, b) = visual.color;
    let fill_color = format!(
        "rgb({},{},{})",
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
    );

    let stroke_color = format!(
        "rgb({},{},{})",
        ((r * 0.6) * 255.0) as u8,
        ((g * 0.6) * 255.0) as u8,
        ((b * 0.6) * 255.0) as u8,
    );

    let opacity = 0.3 + visual.density * 0.7;

    // Build inner pattern lines based on density
    let inner_lines = render_inner_pattern(&pts, visual.density, &stroke_color);

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{w}" viewBox="0 0 {w} {w}">
  <polygon points="{p0x},{p0y} {p1x},{p1y} {p2x},{p2y}"
    fill="{fill}" fill-opacity="{opacity:.2}" stroke="{stroke}" stroke-width="2"/>
  {inner}
</svg>"#,
        w = canvas_size,
        p0x = pts[0].0, p0y = pts[0].1,
        p1x = pts[1].0, p1y = pts[1].1,
        p2x = pts[2].0, p2y = pts[2].1,
        fill = fill_color,
        opacity = opacity,
        stroke = stroke_color,
        inner = inner_lines,
    )
}

/// Generate inner pattern lines based on density.
fn render_inner_pattern(pts: &[(f64, f64)], density: f64, color: &str) -> String {
    let n_lines = (density * 5.0).round() as usize;
    if n_lines == 0 || pts.len() < 3 {
        return String::new();
    }

    let mut lines = String::new();
    let cx = (pts[0].0 + pts[1].0 + pts[2].0) / 3.0;
    let cy = (pts[0].1 + pts[1].1 + pts[2].1) / 3.0;

    for i in 0..n_lines {
        let t = (i as f64 + 1.0) / (n_lines as f64 + 1.0);
        // Interpolate between centroid and each edge midpoint
        let edge_idx = i % 3;
        let next_idx = (edge_idx + 1) % 3;
        let mx = (pts[edge_idx].0 + pts[next_idx].0) / 2.0;
        let my = (pts[edge_idx].1 + pts[next_idx].1) / 2.0;

        let x1 = cx + (mx - cx) * t;
        let y1 = cy + (my - cy) * t;
        let x2 = cx + (pts[edge_idx].0 - cx) * t;
        let y2 = cy + (pts[edge_idx].1 - cy) * t;

        lines.push_str(&format!(
            r#"  <line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{color}" stroke-width="0.5" stroke-opacity="0.4"/>"#,
            x1 = x1, y1 = y1, x2 = x2, y2 = y2, color = color,
        ));
        lines.push('\n');
    }

    lines
}

// ============================================================================
// Cluster Visualization
// ============================================================================

/// Compute visual coordinates for all glyphs in a store.
pub fn visualize_store(store: &GlyphStore) -> Vec<GlyphVisual> {
    let mut visuals = Vec::new();
    for hash in store.list_hashes() {
        if let Some(stored) = store.get(hash) {
            if let Some(visual) = fano_to_visual(&stored.glyph) {
                visuals.push(visual);
            }
        }
    }
    visuals
}

/// Simple k-means-like clustering of glyph visuals by centroid proximity.
///
/// Groups visuals into clusters based on their centroid distance.
/// Uses a single-pass greedy algorithm (not full k-means) for simplicity.
pub fn cluster_visuals(visuals: &[GlyphVisual], distance_threshold: f64) -> Vec<GlyphCluster> {
    let mut clusters: Vec<GlyphCluster> = Vec::new();

    for visual in visuals {
        let mut assigned = false;
        for cluster in &mut clusters {
            let dist = euclidean_distance(visual.centroid, cluster.centroid);
            if dist < distance_threshold {
                // Add to existing cluster and update centroid
                let n = cluster.count as f64;
                cluster.centroid.0 = (cluster.centroid.0 * n + visual.centroid.0) / (n + 1.0);
                cluster.centroid.1 = (cluster.centroid.1 * n + visual.centroid.1) / (n + 1.0);
                cluster.avg_color.0 = (cluster.avg_color.0 * n + visual.color.0) / (n + 1.0);
                cluster.avg_color.1 = (cluster.avg_color.1 * n + visual.color.1) / (n + 1.0);
                cluster.avg_color.2 = (cluster.avg_color.2 * n + visual.color.2) / (n + 1.0);
                cluster.members.push(visual.glyph_hash.clone());
                cluster.count += 1;
                assigned = true;
                break;
            }
        }

        if !assigned {
            clusters.push(GlyphCluster {
                cluster_id: format!("cluster-{}", clusters.len()),
                centroid: visual.centroid,
                members: vec![visual.glyph_hash.clone()],
                avg_color: visual.color,
                count: 1,
            });
        }
    }

    clusters
}

/// Render a collective overview SVG with all glyphs positioned by their visual coordinates.
pub fn render_collective_svg(visuals: &[GlyphVisual], canvas_size: f64) -> String {
    let half = canvas_size / 2.0;
    let scale = canvas_size * 0.3;

    let mut elements = String::new();

    for visual in visuals {
        let cx = half + visual.centroid.0 * scale;
        let cy = half - visual.centroid.1 * scale;
        let r = 5.0 + visual.size * 20.0;
        let (rv, gv, bv) = visual.color;

        elements.push_str(&format!(
            r#"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="rgb({rv},{gv},{bv})" fill-opacity="{op:.2}" stroke="none"/>
"#,
            cx = cx, cy = cy, r = r,
            rv = (rv * 255.0) as u8,
            gv = (gv * 255.0) as u8,
            bv = (bv * 255.0) as u8,
            op = 0.3 + visual.density * 0.5,
        ));
    }

    let bg_color = "#0a0a0a";
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{w}" viewBox="0 0 {w} {w}">
  <rect width="{w}" height="{w}" fill="{bg}"/>
{elements}</svg>"#,
        w = canvas_size,
        bg = bg_color,
        elements = elements,
    )
}

fn euclidean_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

// ============================================================================
// ADR-0015 Phase 7: Universal Glyph Visual Language
// ============================================================================

/// Visual coordinates derived from a universal Glyph's Fano projection.
///
/// Extends the PrivacyGlyph visual with source-type color tinting
/// and SGA-aware positioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalGlyphVisual {
    /// Hex glyph_id
    pub glyph_id: String,
    /// Triangle vertices
    pub vertices: [(f64, f64); 3],
    /// Base RGB color from Fano
    pub color: (f64, f64, f64),
    /// Color tint from source modality
    pub source_tint: (f64, f64, f64),
    /// Final blended color
    pub blended_color: (f64, f64, f64),
    /// Inner pattern density
    pub density: f64,
    /// Centroid position
    pub centroid: (f64, f64),
    /// Visual size (proportional to amplitude)
    pub size: f64,
    /// Source type label
    pub source_type: String,
    /// SGA class index (0-95)
    pub sga_class_index: u8,
}

/// Source type → tint color mapping.
///
/// Each modality gets a distinct color accent so constellation
/// visualizations show modality distribution at a glance.
fn source_tint(source: &GlyphSource) -> (f64, f64, f64) {
    match source {
        GlyphSource::Memory { .. }     => (0.3, 0.5, 0.9),  // Blue — thought
        GlyphSource::Audio { .. }      => (0.9, 0.5, 0.2),  // Orange — sound
        GlyphSource::Visual { .. }     => (0.2, 0.9, 0.4),  // Green — sight
        GlyphSource::Scada { .. }      => (0.8, 0.2, 0.2),  // Red — industrial
        GlyphSource::Financial { .. }  => (0.9, 0.9, 0.2),  // Gold — money
        GlyphSource::Prediction { .. } => (0.6, 0.2, 0.8),  // Purple — foresight
        GlyphSource::Flux { .. }       => (0.2, 0.8, 0.8),  // Cyan — network
        GlyphSource::Dream { .. }      => (0.7, 0.3, 0.7),  // Magenta — dream
        GlyphSource::Other { .. }      => (0.5, 0.5, 0.5),  // Gray — unknown
    }
}

/// Source type label.
fn source_label(source: &GlyphSource) -> &'static str {
    match source {
        GlyphSource::Memory { .. }     => "memory",
        GlyphSource::Audio { .. }      => "audio",
        GlyphSource::Visual { .. }     => "visual",
        GlyphSource::Scada { .. }      => "scada",
        GlyphSource::Financial { .. }  => "financial",
        GlyphSource::Prediction { .. } => "prediction",
        GlyphSource::Flux { .. }       => "flux",
        GlyphSource::Dream { .. }      => "dream",
        GlyphSource::Other { .. }      => "other",
    }
}

/// Convert a universal Glyph to visual coordinates.
///
/// Like `fano_to_visual` for PrivacyGlyph, but adds source-type
/// color tinting and SGA positioning.
pub fn glyph_to_visual(glyph: &Glyph) -> UniversalGlyphVisual {
    let fano = &glyph.fano;

    // Base vertices from Fano[0..3]
    let base_vertices = [
        (0.0, 1.0),
        (-0.866025, -0.5),
        (0.866025, -0.5),
    ];
    let vertices = [
        distort_vertex(base_vertices[0], fano[0], 0),
        distort_vertex(base_vertices[1], fano[1], 1),
        distort_vertex(base_vertices[2], fano[2], 2),
    ];

    // Base color from Fano[3..6]
    let base_color = (
        normalize_fano(fano[3]),
        normalize_fano(fano[4]),
        normalize_fano(fano[5]),
    );

    // Source tint
    let tint = source_tint(&glyph.source);

    // Blend: 60% Fano color + 40% source tint
    let blended_color = (
        (base_color.0 * 0.6 + tint.0 * 0.4).clamp(0.0, 1.0),
        (base_color.1 * 0.6 + tint.1 * 0.4).clamp(0.0, 1.0),
        (base_color.2 * 0.6 + tint.2 * 0.4).clamp(0.0, 1.0),
    );

    let density = normalize_fano(fano[6]);

    let centroid = (
        (vertices[0].0 + vertices[1].0 + vertices[2].0) / 3.0,
        (vertices[0].1 + vertices[1].1 + vertices[2].1) / 3.0,
    );

    let size = glyph.amplitude.abs().min(2.0) / 2.0;

    let glyph_id_hex = glyph.glyph_id.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    UniversalGlyphVisual {
        glyph_id: glyph_id_hex,
        vertices,
        color: base_color,
        source_tint: tint,
        blended_color,
        density,
        centroid,
        size,
        source_type: source_label(&glyph.source).to_string(),
        sga_class_index: glyph.sga_class.to_class_index(),
    }
}

/// Render a constellation SVG showing all universal glyphs.
///
/// Glyphs are positioned by centroid, colored by blended color,
/// and sized by amplitude. Source type is shown via tint.
pub fn render_constellation_svg(visuals: &[UniversalGlyphVisual], canvas_size: f64) -> String {
    let half = canvas_size / 2.0;
    let scale = canvas_size * 0.3;

    let mut elements = String::new();

    for visual in visuals {
        let cx = half + visual.centroid.0 * scale;
        let cy = half - visual.centroid.1 * scale;
        let r = 5.0 + visual.size * 20.0;
        let (rv, gv, bv) = visual.blended_color;
        let opacity = 0.3 + visual.density * 0.5;

        elements.push_str(&format!(
            r#"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="rgb({rv},{gv},{bv})" fill-opacity="{op:.2}" stroke="none"/>
"#,
            cx = cx, cy = cy, r = r,
            rv = (rv * 255.0) as u8,
            gv = (gv * 255.0) as u8,
            bv = (bv * 255.0) as u8,
            op = opacity,
        ));
    }

    let bg_color = "#080810";
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{w}" viewBox="0 0 {w} {w}">
  <rect width="{w}" height="{w}" fill="{bg}"/>
{elements}</svg>"#,
        w = canvas_size,
        bg = bg_color,
        elements = elements,
    )
}

/// Cluster universal glyph visuals by source type.
pub fn cluster_by_source(visuals: &[UniversalGlyphVisual]) -> Vec<(String, Vec<&UniversalGlyphVisual>)> {
    let mut map: std::collections::HashMap<String, Vec<&UniversalGlyphVisual>> =
        std::collections::HashMap::new();
    for v in visuals {
        map.entry(v.source_type.clone()).or_default().push(v);
    }
    let mut result: Vec<_> = map.into_iter().collect();
    result.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collective::privacy::seal_with_commitments;
    use crate::memory::HyperMemory;

    fn test_memory(content: &str) -> HyperMemory {
        HyperMemory::new(vec![0.1; 100], content.to_string())
    }

    fn make_glyph(content: &str, agent: &str) -> PrivacyGlyph {
        let mem = test_memory(content);
        let result = seal_with_commitments(&mem, 0, agent);
        let mut glyph = result.glyph;
        // Ensure Fano projection is set for visual tests.
        // In production, this comes from the memory's geometry.
        if glyph.fano_projection.is_none() {
            // Compute a simple projection from the vector hash
            let hash_val = {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut h = DefaultHasher::new();
                content.hash(&mut h);
                h.finish()
            };
            let mut fano = [0.0f64; 7];
            for i in 0..7 {
                fano[i] = ((hash_val >> (i * 8)) & 0xFF) as f64 / 255.0 * 0.3;
            }
            glyph.fano_projection = Some(fano);
        }
        glyph
    }

    #[test]
    fn test_fano_to_visual_produces_coordinates() {
        let glyph = make_glyph("quantum computing", "alice");
        assert!(glyph.fano_projection.is_some());

        let visual = fano_to_visual(&glyph).unwrap();
        assert_eq!(visual.glyph_hash, glyph.glyph_hash);
        assert_eq!(visual.vertices.len(), 3);

        // Color channels should be in [0, 1]
        assert!(visual.color.0 >= 0.0 && visual.color.0 <= 1.0);
        assert!(visual.color.1 >= 0.0 && visual.color.1 <= 1.0);
        assert!(visual.color.2 >= 0.0 && visual.color.2 <= 1.0);

        // Density should be in [0, 1]
        assert!(visual.density >= 0.0 && visual.density <= 1.0);
    }

    #[test]
    fn test_similar_memories_similar_visuals() {
        // Same content → same vector → same Fano → same visual
        let g1 = make_glyph("identical content", "alice");
        let g2 = make_glyph("identical content", "bob");

        let v1 = fano_to_visual(&g1).unwrap();
        let v2 = fano_to_visual(&g2).unwrap();

        // Same Fano values → same visual coordinates
        assert!((v1.color.0 - v2.color.0).abs() < 1e-10);
        assert!((v1.color.1 - v2.color.1).abs() < 1e-10);
        assert!((v1.color.2 - v2.color.2).abs() < 1e-10);
        assert!((v1.density - v2.density).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_fano_range() {
        // Test various inputs
        assert!(normalize_fano(0.0) >= 0.0 && normalize_fano(0.0) <= 1.0);
        assert!(normalize_fano(0.5) >= 0.0 && normalize_fano(0.5) <= 1.0);
        assert!(normalize_fano(-1.0) >= 0.0 && normalize_fano(-1.0) <= 1.0);
        assert!(normalize_fano(10.0) >= 0.0 && normalize_fano(10.0) <= 1.0);

        // Higher input → higher output (monotonic)
        assert!(normalize_fano(0.3) > normalize_fano(0.1));
        assert!(normalize_fano(0.5) > normalize_fano(0.3));
    }

    #[test]
    fn test_render_svg_valid_output() {
        let glyph = make_glyph("test memory", "alice");
        let visual = fano_to_visual(&glyph).unwrap();
        let svg = render_svg(&visual, 200.0);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("rgb("));
    }

    #[test]
    fn test_visualize_store() {
        let mut store = GlyphStore::new();

        // Use make_glyph helper to ensure fano_projection is set
        let g1 = make_glyph("memory alpha", "alice");
        let g2 = make_glyph("memory beta", "bob");
        store.insert_remote(g1);
        store.insert_remote(g2);

        let visuals = visualize_store(&store);
        assert_eq!(visuals.len(), 2);
    }

    #[test]
    fn test_cluster_visuals_same_location() {
        let g1 = make_glyph("similar topic A", "alice");
        let g2 = make_glyph("similar topic A", "bob"); // Same content → same position

        let v1 = fano_to_visual(&g1).unwrap();
        let v2 = fano_to_visual(&g2).unwrap();

        let clusters = cluster_visuals(&[v1, v2], 0.5);
        // Same content → same centroid → single cluster
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 2);
    }

    #[test]
    fn test_cluster_visuals_empty() {
        let clusters = cluster_visuals(&[], 0.5);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_render_collective_svg() {
        let g1 = make_glyph("memory one", "alice");
        let g2 = make_glyph("memory two", "bob");

        let v1 = fano_to_visual(&g1).unwrap();
        let v2 = fano_to_visual(&g2).unwrap();

        let svg = render_collective_svg(&[v1, v2], 800.0);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("#0a0a0a")); // Dark background
    }

    #[test]
    fn test_glyph_without_fano_returns_none() {
        let mem = test_memory("test");
        let mut glyph = seal_with_commitments(&mem, 0, "alice").glyph;
        glyph.fano_projection = None;
        assert!(fano_to_visual(&glyph).is_none());
    }

    // ── Phase 7: Universal Glyph Visual Language ──

    fn make_universal_glyph(source: GlyphSource) -> Glyph {
        use crate::collective::privacy::BloomParameters;
        Glyph {
            glyph_id: [0u8; 32],
            spec_version: 1,
            fano: [0.14, 0.14, 0.14, 0.14, 0.15, 0.15, 0.14],
            sga_class: crate::collective::glyph_spec::SgaClass { quadrant: 0, modality: 0, context: 0 },
            sga_centroid: (0, 0, 0),
            amplitude: 0.8,
            frequency: 0.5,
            phase: 0.0,
            capsule: None,
            bloom: BloomParameters { difficulty: 0, salt: [0; 32] },
            commitments: None,
            virtue_eta: None,
            gates: None,
            source,
            agent_id: "test".to_string(),
            created_at: chrono::Utc::now(),
            parents: Vec::new(),
        }
    }

    #[test]
    fn test_glyph_to_visual_memory() {
        let glyph = make_universal_glyph(GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let visual = glyph_to_visual(&glyph);
        assert_eq!(visual.source_type, "memory");
        // Memory tint is blue-ish
        assert!(visual.source_tint.2 > visual.source_tint.0, "memory tint should be blue");
        assert_eq!(visual.vertices.len(), 3);
        assert!(visual.blended_color.0 >= 0.0 && visual.blended_color.0 <= 1.0);
    }

    #[test]
    fn test_glyph_to_visual_audio() {
        let glyph = make_universal_glyph(GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });
        let visual = glyph_to_visual(&glyph);
        assert_eq!(visual.source_type, "audio");
        // Audio tint is orange-ish
        assert!(visual.source_tint.0 > visual.source_tint.2, "audio tint should be orange");
    }

    #[test]
    fn test_glyph_to_visual_scada() {
        let glyph = make_universal_glyph(GlyphSource::Scada {
            tag: "TI-101".to_string(), value: 75.0, unit: "degC".to_string(), quality: 192,
        });
        let visual = glyph_to_visual(&glyph);
        assert_eq!(visual.source_type, "scada");
        // SCADA tint is red
        assert!(visual.source_tint.0 > visual.source_tint.1, "scada tint should be red");
    }

    #[test]
    fn test_render_constellation_svg() {
        let g1 = make_universal_glyph(GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let g2 = make_universal_glyph(GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });
        let visuals = vec![glyph_to_visual(&g1), glyph_to_visual(&g2)];
        let svg = render_constellation_svg(&visuals, 800.0);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("#080810")); // Dark background
    }

    #[test]
    fn test_cluster_by_source() {
        let g1 = make_universal_glyph(GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let g2 = make_universal_glyph(GlyphSource::Memory { layer_depth: 1, hallucinated: false });
        let g3 = make_universal_glyph(GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });
        let visuals = vec![glyph_to_visual(&g1), glyph_to_visual(&g2), glyph_to_visual(&g3)];
        let clusters = cluster_by_source(&visuals);
        assert_eq!(clusters.len(), 2);
        // Memory cluster should be first (more members)
        assert_eq!(clusters[0].0, "memory");
        assert_eq!(clusters[0].1.len(), 2);
        assert_eq!(clusters[1].0, "audio");
        assert_eq!(clusters[1].1.len(), 1);
    }

    #[test]
    fn test_visual_size_proportional_to_amplitude() {
        let mut g1 = make_glyph("low amplitude", "alice");
        let mut g2 = make_glyph("high amplitude", "bob");
        g1.committed_amplitude = 0.2;
        g2.committed_amplitude = 1.8;

        let v1 = fano_to_visual(&g1).unwrap();
        let v2 = fano_to_visual(&g2).unwrap();

        assert!(v2.size > v1.size);
    }
}
