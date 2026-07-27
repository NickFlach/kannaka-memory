//! Recovering the actual question from a harness-assembled prompt.
//!
//! `buzz-acp` does not hand an agent a bare question. It assembles a sectioned
//! prompt — `[Context]`, `[Thread Context]`, `[Agent Memory]`, then an event
//! block per triggering event:
//!
//! ```text
//! [Context]
//! Scope: channel
//! Channel: general (#<uuid>)
//! IMPORTANT: ... use `--reply-to <id>` ...
//!
//! [Event]
//! Event ID: <hex>
//! Channel: general (#<uuid>)
//! Kind: 9
//! From: Nick (npub: ..., hex: ...)
//! Time: 2026-07-27T06:00:00+00:00
//! Content: what do you remember about the radio?
//! Tags: [["p","..."]]
//! Parsed: root=...
//! ```
//!
//! Resonating that whole blob through the medium is actively harmful: the
//! scaffolding dominates the query vector, so recall returns memories matching
//! "Context/Channel/Event" boilerplate instead of the question — and the
//! rendered answer echoes the harness internals back into the channel.
//!
//! So the query is the `Content:` field of the **last** event block. Last,
//! because a batch may carry several events and the final one is the one being
//! responded to — the same rule `buzz-acp` uses to derive scope.

/// Marker introducing the message body inside an event block.
const CONTENT: &str = "Content: ";

/// Field lines that terminate a `Content:` body. `Tags:` is always emitted by
/// the harness, so it is the reliable terminator; `Parsed:` is conditional.
const TERMINATORS: [&str; 2] = ["Tags: ", "Parsed: "];

/// Extract the question to resonate from a harness-assembled prompt.
///
/// Falls back to the whole prompt when there is no event block — that is the
/// direct-prompt case (the Buzz desktop harness gallery, or a manual `--top-k`
/// smoke test), where the prompt already *is* the question.
pub(crate) fn extract_query(prompt: &str) -> String {
    let Some(start) = last_content_start(prompt) else {
        return prompt.trim().to_string();
    };

    let body = &prompt[start..];
    let mut kept: Vec<&str> = Vec::new();
    for (i, line) in body.split('\n').enumerate() {
        // The first line is the remainder of the `Content:` line itself and can
        // never be a terminator, however it happens to begin.
        if i > 0 && is_boundary(line) {
            break;
        }
        kept.push(line);
    }
    kept.join("\n").trim().to_string()
}

/// Byte offset just past the last line-initial `Content: ` marker.
fn last_content_start(prompt: &str) -> Option<usize> {
    if let Some(i) = prompt.rfind(&format!("\n{CONTENT}")) {
        return Some(i + 1 + CONTENT.len());
    }
    // An event block can also be the very start of the prompt.
    prompt.starts_with(CONTENT).then_some(CONTENT.len())
}

/// True when `line` starts a new harness field or section, ending the body.
fn is_boundary(line: &str) -> bool {
    line.starts_with('[') || TERMINATORS.iter().any(|t| line.starts_with(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A realistic single-event prompt in the harness's exact shape.
    fn harness_prompt(content: &str) -> String {
        format!(
            "[Context]\n\
             Scope: channel\n\
             Channel: general (#8f14e45f-ceea-467a-9c1e-1b2c3d4e5f60)\n\
             IMPORTANT: use `--reply-to {id}` on `buzz messages send`.\n\
             \n\
             [Event]\n\
             Event ID: {id}\n\
             Channel: general (#8f14e45f-ceea-467a-9c1e-1b2c3d4e5f60)\n\
             Kind: 9\n\
             From: Nick (npub: npub1abc, hex: abc)\n\
             Time: 2026-07-27T06:00:00+00:00\n\
             Content: {content}\n\
             Tags: [[\"p\",\"abc\"]]\n\
             Parsed: root={id}",
            id = "1".repeat(64),
            content = content,
        )
    }

    #[test]
    fn extracts_only_the_message_content() {
        let got = extract_query(&harness_prompt("what do you remember about the radio?"));
        assert_eq!(got, "what do you remember about the radio?");
    }

    #[test]
    fn drops_the_context_scaffolding_entirely() {
        // The bug this module exists to fix: scaffolding leaking into the query
        // dominates the resonance vector and is echoed back to the channel.
        let got = extract_query(&harness_prompt("kannaka radio"));
        for leaked in ["[Context]", "Scope:", "Channel:", "--reply-to", "Event ID:"] {
            assert!(!got.contains(leaked), "{leaked:?} leaked into query: {got:?}");
        }
    }

    #[test]
    fn keeps_multiline_content_together() {
        let got = extract_query(&harness_prompt("first line\nsecond line"));
        assert_eq!(got, "first line\nsecond line");
    }

    #[test]
    fn stops_at_the_tags_field() {
        let got = extract_query(&harness_prompt("a question"));
        assert!(!got.contains("Tags:"), "got: {got:?}");
        assert!(!got.contains("Parsed:"), "got: {got:?}");
    }

    #[test]
    fn uses_the_last_event_in_a_batch() {
        // The final event is the one being responded to.
        let prompt = format!(
            "[Buzz events]\n\
             Event ID: a\nContent: older question\nTags: []\n\
             Event ID: b\nContent: newest question\nTags: []"
        );
        assert_eq!(extract_query(&prompt), "newest question");
    }

    #[test]
    fn bare_prompt_without_an_event_block_is_used_as_is() {
        // Desktop harness gallery / manual smoke test.
        assert_eq!(extract_query("  what is kannaka?  "), "what is kannaka?");
    }

    #[test]
    fn content_at_the_very_start_is_found() {
        assert_eq!(extract_query("Content: hello\nTags: []"), "hello");
    }

    #[test]
    fn empty_content_yields_empty_query() {
        // Caller treats this as "no query" rather than resonating whitespace.
        assert_eq!(extract_query("Content: \nTags: []"), "");
    }

    #[test]
    fn content_mentioning_tags_on_its_first_line_is_not_truncated() {
        let got = extract_query(&harness_prompt("Tags: are confusing"));
        assert_eq!(got, "Tags: are confusing");
    }

    #[test]
    fn a_later_section_header_ends_the_body() {
        let prompt = "[Event]\nContent: the question\n[Something Else]\nignored";
        assert_eq!(extract_query(prompt), "the question");
    }
}
