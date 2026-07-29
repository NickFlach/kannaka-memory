//! Presentation: turning recalled memories into the agent's answer text.
//!
//! Split from dispatch so the wire protocol and the wording of answers can
//! change independently. This module is pure formatting — no I/O, no protocol.

use super::server::Recollection;

/// Render recalled memories as the agent's answer.
pub(crate) fn render(query: &str, hits: &[Recollection]) -> String {
    if hits.is_empty() {
        return format!("No memories resonated with \"{query}\".");
    }

    let mut out = format!(
        "{} {} for \"{}\":\n",
        hits.len(),
        if hits.len() == 1 { "memory" } else { "memories" },
        query
    );
    for (i, hit) in hits.iter().enumerate() {
        out.push_str(&format!(
            "\n{}. [{:.0}% · {}] {}",
            i + 1,
            hit.similarity * 100.0,
            format_age(hit.age_hours),
            hit.content.trim()
        ));
    }
    out
}

/// Human-readable age, coarsened by magnitude.
fn format_age(hours: f64) -> String {
    if hours < 1.0 {
        "just now".to_string()
    } else if hours < 24.0 {
        format!("{}h ago", hours.round() as i64)
    } else {
        format!("{}d ago", (hours / 24.0).round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(content: &str, similarity: f32, age_hours: f64) -> Recollection {
        Recollection {
            content: content.to_string(),
            similarity,
            age_hours,
        }
    }

    #[test]
    fn formats_rank_score_and_age() {
        let text = render("q", &[hit("alpha", 0.9, 0.5), hit("beta", 0.5, 48.0)]);
        assert!(text.contains("2 memories"), "got: {text}");
        assert!(text.contains("1. [90% · just now] alpha"), "got: {text}");
        assert!(text.contains("2. [50% · 2d ago] beta"), "got: {text}");
    }

    #[test]
    fn uses_singular_for_one_hit() {
        let text = render("q", &[hit("only", 1.0, 3.0)]);
        assert!(text.contains("1 memory for"), "got: {text}");
        assert!(text.contains("3h ago"), "got: {text}");
    }

    #[test]
    fn no_hits_names_the_query() {
        let text = render("nostr membrane", &[]);
        assert!(text.contains("No memories resonated"), "got: {text}");
        assert!(text.contains("nostr membrane"), "got: {text}");
    }

    #[test]
    fn content_is_trimmed_so_ranks_stay_aligned() {
        let text = render("q", &[hit("  padded  ", 0.5, 1.0)]);
        assert!(text.contains("] padded"), "got: {text}");
    }

    #[test]
    fn age_boundaries_switch_units() {
        // <1h, exactly 1h, and the 24h day boundary.
        assert_eq!(format_age(0.9), "just now");
        assert_eq!(format_age(1.0), "1h ago");
        assert_eq!(format_age(23.4), "23h ago");
        assert_eq!(format_age(24.0), "1d ago");
    }
}
