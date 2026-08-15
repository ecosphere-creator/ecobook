//! Portable markdown deck-document format: import/export for `SlideDeck`.
//!
//! A deck document is a single markdown file with YAML frontmatter. It is
//! authored as a 16:9 presentation (1920x1080 canvas) so the same document
//! can be imported into any ecobook instance and rendered both by the
//! presentation player and by the responsive `/ecobook` portrait reader.
//!
//! ## Document shape
//!
//! ```markdown
//! ---
//! deck:
//!   name: "Ecosphere — The Software Composition Platform"
//!   slug: ecosphere-investor-pitch
//!   subtitle: "The Software Composition Platform for the AI era"
//!   level: "Intermediate"
//!   language: "en"
//!   instructorName: "Ecosphere"
//!   tags: [pitch, ecosphere, investor]
//!   status: draft
//! theme:
//!   base: light
//!   bg: "#f7f6f2"
//!   ink: "#17141d"
//!   accent: "#5b3fd6"
//!   fontDisplay: "Manrope, ui-sans-serif, system-ui, sans-serif"
//!   fontMono: "DM Mono, ui-monospace, monospace"
//! ---
//!
//! # The next wave of lean infrastructure
//!
//! A paragraph becomes a body element.
//!
//! > A quote or key stat becomes a callout element.
//!
//! ```text
//! code / pipeline becomes a code element
//! ```
//! ```
//!
//! ## Mapping rules
//!
//! - `#` heading (H1) starts a new slide; the heading text is the slide title
//!   element. `##`/`###` become level-2/3 sub-title elements on the same slide.
//! - A bare paragraph becomes a `text`/`paragraph` element.
//! - `> blockquote` becomes a `callout` element.
//! - A fenced code block becomes a `paragraph` element with `whiteSpace: pre`
//!   + monospace font (renders as a code block).
//! - A `---` horizontal rule ends the current slide (a deck can have multiple
//!   H1 blocks, but `---` is an explicit slide break).
//! - `**bold**`/`*italic*` markers are preserved in `content` verbatim — the
//!   renderers already understand them.
//!
//! All elements are laid out on the 1920x1080 canvas with an auto-stacking
//! layout engine (title at top, then blocks flowing downward), so imported
//! decks never have physically overlapping elements.

use crate::{
    dto::SlideDeckInput,
    error::{AppError, AppResult},
    models::slide_deck::{Element, Slide},
};

pub const CANVAS_W: f64 = 1920.0;
pub const CANVAS_H: f64 = 1080.0;

/// Default getecosphere design tokens (deep-violet anchor, Manrope/DM Mono).
/// Used when a document omits the `theme:` block entirely.
fn default_theme() -> DeckTheme {
    DeckTheme {
        base: "light".to_string(),
        bg: "#f7f6f2".to_string(),
        ink: "#17141d".to_string(),
        accent: "#5b3fd6".to_string(),
        surface: "#ffffff".to_string(),
        font_display: "Manrope, ui-sans-serif, system-ui, sans-serif".to_string(),
        font_mono: "DM Mono, ui-monospace, monospace".to_string(),
    }
}

/// Theme tokens carried by a deck document.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeckTheme {
    pub base: String,
    pub bg: String,
    pub ink: String,
    pub accent: String,
    pub surface: String,
    #[serde(default = "default_display_font")]
    pub font_display: String,
    #[serde(default = "default_mono_font")]
    pub font_mono: String,
}

fn default_display_font() -> String {
    "Manrope, ui-sans-serif, system-ui, sans-serif".to_string()
}
fn default_mono_font() -> String {
    "DM Mono, ui-monospace, monospace".to_string()
}

/// Deck metadata from the frontmatter `deck:` block. Optional fields map to
/// `SlideDeckInput` one-to-one; anything the author leaves out is absent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeckMeta {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub long_summary: Option<String>,
    #[serde(default)]
    pub learning_objectives: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub target_audience: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub level: Option<String>,
    pub language: Option<String>,
    pub instructor_name: Option<String>,
    pub estimated_duration_minutes: Option<i32>,
    pub status: Option<String>,
    pub cover_url: Option<String>,
}

/// A slide produced by the markdown parser: title/level + ordered blocks.
struct ParsedSlide {
    name: String,
    level: i32,
    /// (element kind, markdown text) — `"title" | "subtitle" | "para" | "callout" | "code"`
    blocks: Vec<(&'static str, String)>,
}

// ---------------------------------------------------------------------------
// Frontmatter parsing
// ---------------------------------------------------------------------------

/// Split a document into (frontmatter, markdown body). Accepts a leading
/// `---` delimited YAML block; a document without frontmatter is all body.
fn split_frontmatter(text: &str) -> (Option<String>, &str) {
    let trimmed = text.trim_start_matches('\u{feff}');
    let rest = trimmed.strip_prefix("---\n").or_else(|| trimmed.strip_prefix("---\r\n"));
    let Some(after_open) = rest else {
        return (None, text);
    };
    // closing `---` must appear on its own line before EOF
    if let Some(end) = after_open.find("\n---\n") {
        let yaml = &after_open[..end];
        let body = &after_open[end + 5..];
        return (Some(yaml.to_string()), body);
    }
    if let Some(end) = after_open.find("\n---") {
        let candidate = &after_open[end + 4..];
        if candidate.trim().is_empty() {
            return (Some(after_open[..end].to_string()), "");
        }
    }
    // no valid closing delimiter: treat whole thing as body
    (None, text)
}

#[derive(Debug, serde::Deserialize)]
struct Frontmatter {
    #[serde(default)]
    deck: DeckMeta,
    #[serde(default)]
    theme: Option<DeckTheme>,
}

fn parse_frontmatter(yaml: &str) -> AppResult<Frontmatter> {
    serde_yaml::from_str(yaml).map_err(|e| AppError::BadRequest(format!("Invalid YAML frontmatter: {e}")))
}

// ---------------------------------------------------------------------------
// Markdown parsing
// ---------------------------------------------------------------------------

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

fn fence_tick(line: &str) -> &str {
    let t = line.trim_start();
    if t.starts_with("```") {
        "```"
    } else {
        "~~~"
    }
}

fn parse_markdown(body: &str) -> AppResult<Vec<ParsedSlide>> {
    let mut slides: Vec<ParsedSlide> = Vec::new();
    let mut current: Option<ParsedSlide> = None;

    // Content blocks (paragraph/callout/code/list) append to the open slide.
    // A slide only closes on a `#` heading, a `---` rule, or EOF.
    fn open_slide<'a>(current: &'a mut Option<ParsedSlide>) -> AppResult<&'a mut ParsedSlide> {
        if current.is_none() {
            return Err(AppError::BadRequest(
                "Content appears before any `#` slide heading — every slide must start with an H1".to_string(),
            ));
        }
        Ok(current.as_mut().unwrap())
    }
    fn close_slide(slides: &mut Vec<ParsedSlide>, current: &mut Option<ParsedSlide>) {
        if let Some(slide) = current.take() {
            slides.push(slide);
        }
    }

    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let raw = lines[i];
        let line = raw.trim_end_matches('\r');
        let trimmed = line.trim();

        // fenced code block: consume until the matching fence
        if is_fence(line) {
            let marker = fence_tick(line).to_string();
            let mut buf = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with(marker.as_str()) {
                buf.push_str(lines[i]);
                buf.push('\n');
                i += 1;
            }
            i += 1; // skip closing fence
            open_slide(&mut current)?.blocks.push(("code", buf.trim_end().to_string()));
            continue;
        }

        // `#` heading closes the previous slide and starts a new one
        if let Some(rest) = trimmed.strip_prefix("# ") {
            close_slide(&mut slides, &mut current);
            current = Some(ParsedSlide {
                name: rest.to_string(),
                level: 1,
                blocks: vec![("title", rest.to_string())],
            });
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            open_slide(&mut current)?.blocks.push(("subtitle", rest.to_string()));
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            open_slide(&mut current)?.blocks.push(("subtitle", rest.to_string()));
            i += 1;
            continue;
        }

        // horizontal rule = explicit slide break
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            close_slide(&mut slides, &mut current);
            i += 1;
            continue;
        }

        // blockquote = callout
        if let Some(rest) = trimmed.strip_prefix('>') {
            let quote = rest.trim_start().trim_start_matches('>').trim().to_string();
            open_slide(&mut current)?.blocks.push(("callout", quote));
            i += 1;
            continue;
        }

        // bullet list — accumulate as a single multi-line body block
        if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            let mut buf = format!("• {rest}\n");
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim_end_matches('\r').trim();
                if let Some(item) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
                    buf.push_str("• ");
                    buf.push_str(item);
                    buf.push('\n');
                    i += 1;
                } else if l.is_empty() || is_fence(lines[i]) || l.starts_with("#") || l == "---" {
                    break;
                } else {
                    // continuation line of a wrapped list item
                    buf.push_str(l);
                    buf.push('\n');
                    i += 1;
                }
            }
            open_slide(&mut current)?.blocks.push(("para", buf.trim_end().to_string()));
            continue;
        }

        // ordinary paragraph — accumulate until a blank line or a structural line
        if !trimmed.is_empty() {
            let mut buf = String::new();
            while i < lines.len() {
                let l = lines[i].trim_end_matches('\r');
                let t = l.trim();
                if t.is_empty() || is_fence(l) || t.starts_with('#') || t == "---" || t == "***" || t == "___" {
                    break;
                }
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(t);
                i += 1;
            }
            open_slide(&mut current)?.blocks.push(("para", buf));
            continue;
        }

        i += 1;
    }

    close_slide(&mut slides, &mut current);
    if slides.is_empty() {
        return Err(AppError::BadRequest("No slides found in document: add at least one `#` heading".to_string()));
    }
    Ok(slides)
}

// ---------------------------------------------------------------------------
// Layout engine — stack blocks on the 1920x1080 canvas
// ---------------------------------------------------------------------------

const MARGIN_X: f64 = 110.0;
const CONTENT_W: f64 = CANVAS_W - MARGIN_X * 2.0; // 1700
const TITLE_Y: f64 = 70.0;
const BODY_TOP: f64 = 210.0;
const BLOCK_GAP: f64 = 26.0;
const FONT_TITLE: f64 = 72.0;
const FONT_SUBTITLE: f64 = 48.0;
const FONT_BODY: f64 = 34.0;
const FONT_CALLOUT: f64 = 40.0;
const FONT_CODE: f64 = 26.0;
const LINE_TITLE: f64 = 1.15;
const LINE_BODY: f64 = 1.5;

/// Estimate rendered height of a block on the canvas (used to stack elements
/// so they never overlap). Assumes ~0.55em average advance width.
fn estimate_lines(text: &str, font_size: f64, width: f64) -> f64 {
    if text.is_empty() {
        return 1.0;
    }
    let avg_char = font_size * 0.55;
    let per_line = (width / avg_char).max(1.0);
    let lines = (text.chars().count() as f64 / per_line).ceil().max(1.0);
    // +0.2 headroom so wrapped last lines don't collide with the next block
    lines + 0.2
}

fn block_height(kind: &str, text: &str, _theme: &DeckTheme) -> f64 {
    let (font_size, line_height, pad_top, pad_bottom) = match kind {
        "title" => (FONT_TITLE, LINE_TITLE * FONT_TITLE, 0.0, 0.0),
        "subtitle" => (FONT_SUBTITLE, 1.2 * FONT_SUBTITLE, 0.0, 0.0),
        "callout" => (FONT_CALLOUT, 1.4 * FONT_CALLOUT, 30.0, 30.0),
        "code" => (FONT_CODE, 1.5 * FONT_CODE, 22.0, 22.0),
        _ => (FONT_BODY, LINE_BODY * FONT_BODY, 0.0, 0.0),
    };
    let width = if kind == "callout" { CONTENT_W - 20.0 } else { CONTENT_W };
    let lines = estimate_lines(text, font_size, width);
    lines * line_height + pad_top + pad_bottom
}

fn style_json(kind: &str, theme: &DeckTheme, is_title_slide: bool) -> String {
    let (font_size, color, weight, font_family, extra) = match kind {
        "title" => (
            FONT_TITLE,
            &theme.accent,
            800,
            Some(theme.font_display.as_str()),
            if is_title_slide {
                format!(r#","displayEffect":"fade-down""#)
            } else {
                format!(r#","textAlign":"left","displayEffect":"fade-down""#)
            },
        ),
        "subtitle" => (
            FONT_SUBTITLE,
            &theme.ink,
            600,
            Some(theme.font_display.as_str()),
            format!(r#","textAlign":"left","displayEffect":"fade""#),
        ),
        "callout" => (
            FONT_CALLOUT,
            &theme.accent,
            700,
            None,
            format!(r#","fontStyle":"italic","borderLeft":"6px solid {}","paddingLeft":28,"displayEffect":"pop""#, theme.accent),
        ),
        "code" => (
            FONT_CODE,
            &theme.ink,
            400,
            Some(theme.font_mono.as_str()),
            format!(r#","whiteSpace":"pre","backgroundColor":"{}","borderRadius":12,"padding":22,"displayEffect":"fade""#, theme.surface),
        ),
        _ => (
            FONT_BODY,
            &theme.ink,
            400,
            None,
            format!(r#","lineHeight":{},"displayEffect":"fade""#, (LINE_BODY * FONT_BODY).round()),
        ),
    };
    let font = font_family.map(|f| format!(r#","fontFamily":"{f}""#)).unwrap_or_default();
    format!(r#"{{"fontSize":{font_size},"color":"{color}","fontWeight":{weight}{font}{extra}}}"#)
}

fn new_element_id(counter: &mut u64) -> String {
    *counter += 1;
    format!("el{counter}")
}

fn build_elements(parsed: &ParsedSlide, theme: &DeckTheme, counter: &mut u64) -> Vec<Element> {
    let mut elements = Vec::new();
    let mut y = if parsed.blocks.first().map(|b| b.0) == Some("title") { TITLE_Y } else { BODY_TOP };

    for (kind, text) in &parsed.blocks {
        let height = block_height(kind, text, theme);
        let is_title_slide = *kind == "title" && parsed.level == 1 && y == TITLE_Y;
        let style = style_json(kind, theme, is_title_slide);

        let x = if *kind == "callout" { MARGIN_X + 20.0 } else { MARGIN_X };
        let width = if *kind == "callout" { CONTENT_W - 20.0 } else { CONTENT_W };

        let element_type = match *kind {
            "callout" => "callout",
            _ => "text",
        };

        elements.push(Element {
            id: new_element_id(counter),
            element_type: Some(element_type.to_string()),
            x,
            y,
            width,
            height,
            content: Some(text.clone()),
            label: None,
            style: Some(style),
            link_url: None,
            flip_h: None,
            flip_v: None,
        });
        y += height + BLOCK_GAP;
    }

    // The title element on a normal content slide doubles as the chapter
    // heading the responsive reader extracts; leave as-is.
    elements
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a deck-document markdown string into a `SlideDeckInput` ready for
/// `create`-style persistence. The cover slide (level 0) is synthesized from
/// frontmatter metadata; each `#` heading adds a content slide.
pub fn parse_document(text: &str) -> AppResult<SlideDeckInput> {
    let (frontmatter_yaml, body) = split_frontmatter(text);
    let meta = match frontmatter_yaml.as_deref() {
        Some(yaml) => parse_frontmatter(yaml)?.deck,
        None => DeckMeta::default(),
    };
    let theme = frontmatter_yaml
        .as_deref()
        .and_then(|yaml| parse_frontmatter(yaml).ok().and_then(|fm| fm.theme))
        .unwrap_or_else(default_theme);

    let parsed_slides = parse_markdown(body)?;

    let mut slides: Vec<Slide> = Vec::new();
    let mut counter = 0u64;

    // Slide 0: cover/title slide (level 0) — synthesized from frontmatter.
    let cover_name = meta.name.clone().unwrap_or_else(|| "Untitled deck".to_string());
    let cover_blocks: Vec<(&'static str, String)> = {
        let mut v = vec![("title", cover_name.clone())];
        if let Some(sub) = meta.subtitle.clone().filter(|s| !s.trim().is_empty()) {
            v.push(("subtitle", sub));
        }
        v
    };
    slides.push(Slide {
        id: "s1".to_string(),
        name: Some("title".to_string()),
        elements: build_elements(
            &ParsedSlide { name: cover_name.clone(), level: 0, blocks: cover_blocks },
            &theme,
            &mut counter,
        ),
        background: Some(theme.bg.clone()),
        level: Some(0),
        progressive_reveal: None,
        reveal_order: None,
        guided_reveal: None,
        rpp: None,
    });

    // Content slides
    let mut slide_no = 1usize;
    for ps in &parsed_slides {
        slide_no += 1;
        slides.push(Slide {
            id: format!("s{slide_no}"),
            name: Some(ps.name.clone()),
            elements: build_elements(ps, &theme, &mut counter),
            background: Some(theme.bg.clone()),
            level: Some(ps.level),
            progressive_reveal: None,
            reveal_order: None,
            guided_reveal: None,
            rpp: None,
        });
    }

    let mut input = SlideDeckInput {
        name: Some(cover_name),
        slug: meta.slug,
        subtitle: meta.subtitle,
        cover_url: meta.cover_url,
        description: meta.description,
        long_summary: meta.long_summary,
        learning_objectives: meta.learning_objectives,
        requirements: meta.requirements,
        target_audience: meta.target_audience,
        tags: meta.tags,
        level: meta.level,
        language: meta.language,
        instructor_name: meta.instructor_name,
        estimated_duration_minutes: meta.estimated_duration_minutes,
        status: meta.status,
        community_id: None,
        event_id: None,
        theme: Some(theme.base.clone()),
        transition: None,
        layout_format: Some("presentation".to_string()),
        price: None,
        compare_at_price: None,
        paywall_start_slide_index: None,
        slides,
        flow: None,
        gallery_images: Vec::new(),
        guided_audio_library: Vec::new(),
    };

    // The cover slide's title element is the deck title; also keep a compact
    // theme descriptor the responsive reader can apply. Reader theming reads
    // `theme` as a JSON blob of tokens (falling back to the named base).
    let theme_json = serde_json::to_string(&theme).unwrap_or_else(|_| theme.base.clone());
    input.theme = Some(theme_json);

    Ok(input)
}

/// Serialize a `SlideDeck` back into the portable markdown document format
/// (the inverse of [`parse_document`]). Used by `GET /book/:id/export`.
pub fn serialize_deck(deck: &crate::models::slide_deck::SlideDeck) -> String {
    let name = deck.name.clone().unwrap_or_else(|| "Untitled deck".to_string());
    let theme = default_theme_from_deck(deck);

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("deck:\n  name: {:?}\n", name));
    if let Some(v) = &deck.slug {
        out.push_str(&format!("  slug: {:?}\n", v));
    }
    if let Some(v) = &deck.subtitle {
        out.push_str(&format!("  subtitle: {:?}\n", v));
    }
    if let Some(v) = &deck.description {
        out.push_str(&format!("  description: {:?}\n", v));
    }
    if let Some(v) = &deck.level {
        out.push_str(&format!("  level: {:?}\n", v));
    }
    if let Some(v) = &deck.language {
        out.push_str(&format!("  language: {:?}\n", v));
    }
    if let Some(v) = &deck.instructor_name {
        out.push_str(&format!("  instructorName: {:?}\n", v));
    }
    if !deck.tags.is_empty() {
        out.push_str(&format!("  tags: [{}]\n", deck.tags.iter().map(|t| format!("{t:?}")).collect::<Vec<_>>().join(", ")));
    }
    out.push_str(&format!("  status: {:?}\n", deck.status));
    out.push_str("theme:\n");
    out.push_str(&format!("  base: {}\n", theme.base));
    out.push_str(&format!("  bg: {:?}\n", theme.bg));
    out.push_str(&format!("  ink: {:?}\n", theme.ink));
    out.push_str(&format!("  accent: {:?}\n", theme.accent));
    out.push_str(&format!("  surface: {:?}\n", theme.surface));
    out.push_str("---\n\n");

    for slide in &deck.slides {
        if slide.level == Some(0) {
            continue; // cover slide is synthesized from frontmatter
        }
        if let Some(title_el) = slide.elements.iter().find(|el| el.element_type.as_deref() == Some("text")) {
            out.push_str(&format!("# {}\n\n", title_el.content.clone().unwrap_or_default()));
        } else if let Some(name) = &slide.name {
            out.push_str(&format!("# {name}\n\n"));
        }
        for el in &slide.elements {
            let Some(content) = el.content.as_deref().map(str::trim).filter(|c| !c.is_empty()) else {
                continue;
            };
            let is_title = matches!(el.element_type.as_deref(), Some("text"))
                && el.y <= 120.0
                && el.x <= MARGIN_X + 20.0;
            if is_title {
                continue;
            }
            let st = el.style.clone().unwrap_or_default();
            match el.element_type.as_deref() {
                Some("callout") => out.push_str(&format!("> {content}\n\n")),
                Some("text" | "paragraph") if st.contains("whiteSpace") || st.contains("monospace") || st.contains("backgroundColor") => {
                    out.push_str("```\n");
                    out.push_str(content);
                    out.push_str("\n```\n\n");
                }
                _ => out.push_str(&format!("{content}\n\n")),
            }
        }
    }

    out
}

/// Rebuild a `DeckTheme` from a stored deck. The import writes the token blob
/// into `theme`; fall back to defaults if it isn't JSON.
fn default_theme_from_deck(deck: &crate::models::slide_deck::SlideDeck) -> DeckTheme {
    deck.theme
        .as_deref()
        .and_then(|t| serde_json::from_str(t).ok())
        .unwrap_or_else(default_theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"---
deck:
  name: "Test Deck"
  slug: test-deck
  subtitle: "A subtitle"
  tags: [one, two]
theme:
  base: light
  bg: "#ffffff"
  ink: "#111111"
  accent: "#5b3fd6"
  surface: "#efecf8"
  fontDisplay: "Manrope, ui-sans-serif, system-ui, sans-serif"
  fontMono: "DM Mono, ui-monospace, monospace"
---

# First slide title

This is the body paragraph.

> A callout quote.

```text
println!("hi");
```

## A subheading

A second paragraph.

---

# Second slide

Some content.
"##;

    #[test]
    fn parses_frontmatter_and_slides() {
        let input = parse_document(SAMPLE).unwrap();
        assert_eq!(input.name.as_deref(), Some("Test Deck"));
        assert_eq!(input.slug.as_deref(), Some("test-deck"));
        assert_eq!(input.tags, vec!["one".to_string(), "two".to_string()]);
        // cover + 2 content slides
        assert_eq!(input.slides.len(), 3);
        assert_eq!(input.slides[0].level, Some(0));
        assert_eq!(input.slides[1].level, Some(1));
        assert_eq!(input.slides[2].level, Some(1));
    }

    #[test]
    fn layout_does_not_overlap_elements() {
        let input = parse_document(SAMPLE).unwrap();
        for (i, slide) in input.slides.iter().enumerate() {
            let mut prev_bottom: Option<f64> = None;
            let mut els: Vec<_> = slide.elements.iter().collect();
            els.sort_by(|a, b| a.y.total_cmp(&b.y));
            for el in els {
                if let Some(prev) = prev_bottom {
                    assert!(
                        el.y >= prev - 0.01,
                        "slide {i}: element '{}' at y={} overlaps previous bottom {prev}",
                        el.content.clone().unwrap_or_default(),
                        el.y
                    );
                }
                // verify the element's own stored height keeps it below the
                // next element (elements never physically overlap on canvas)
                let h = el.height.max(0.0);
                prev_bottom = Some(el.y + h);
            }
        }
    }

    #[test]
    fn round_trips_through_serialize() {
        let input = parse_document(SAMPLE).unwrap();
        let deck = crate::models::slide_deck::SlideDeck {
            id: None,
            name: input.name.clone(),
            slug: input.slug.clone(),
            subtitle: input.subtitle.clone(),
            cover_url: input.cover_url.clone(),
            description: input.description.clone(),
            long_summary: input.long_summary.clone(),
            learning_objectives: input.learning_objectives.clone(),
            requirements: input.requirements.clone(),
            target_audience: input.target_audience.clone(),
            tags: input.tags.clone(),
            level: input.level.clone(),
            language: input.language.clone(),
            instructor_name: input.instructor_name.clone(),
            estimated_duration_minutes: input.estimated_duration_minutes,
            status: input.status.clone().unwrap_or_else(|| "draft".to_string()),
            owner_id: "owner".to_string(),
            community_id: None,
            event_id: None,
            theme: input.theme.clone(),
            transition: None,
            layout_format: Some("presentation".to_string()),
            price: None,
            compare_at_price: None,
            paywall_start_slide_index: None,
            slides: input.slides.clone(),
            flow: None,
            gallery_images: Vec::new(),
            guided_audio_library: Vec::new(),
            created_at: bson::DateTime::now(),
            updated_at: bson::DateTime::now(),
        };
        let doc = serialize_deck(&deck);
        // reparse the exported document and verify the shape survives
        let reparsed = parse_document(&doc).unwrap();
        assert_eq!(reparsed.name, input.name);
        assert_eq!(reparsed.slides.len(), input.slides.len());
        // cover titles match
        assert_eq!(reparsed.slides[0].elements[0].content, input.slides[0].elements[0].content);
    }

    #[test]
    fn empty_document_errors() {
        assert!(parse_document("no headings here\njust text").is_err());
    }

    #[test]
    fn parses_the_investor_pitch_document() {
        // Real authored document (getecosphere_composition/docs). Path is
        // relative to the backend crate root.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../getecosphere_composition/docs/ecosphere-investor-pitch.md");
        let doc = std::fs::read_to_string(&path).expect("pitch document exists");
        let input = match parse_document(&doc) {
            Ok(v) => v,
            Err(e) => panic!("parse failed: {e:?}"),
        };

        // 1 cover + 13 content slides = 14 total (the agreed pitch length)
        assert_eq!(input.slides.len(), 14, "expected 14 slides (cover + 13)");
        assert_eq!(input.slides[0].level, Some(0));
        assert_eq!(input.name.as_deref(), Some("Ecosphere — The Software Composition Platform"));

        // every slide title is distinct (no overlapping messages) and every
        // element is positioned on the canvas without overlapping its siblings
        let mut titles: std::collections::HashSet<String> = std::collections::HashSet::new();
        for slide in &input.slides {
            let first = slide.elements.first().map(|e| e.content.clone().unwrap_or_default()).unwrap_or_default();
            assert!(titles.insert(first.clone()), "duplicate slide title: {first:?}");
            let mut els: Vec<_> = slide.elements.iter().collect();
            els.sort_by(|a, b| a.y.total_cmp(&b.y));
            let mut prev_bottom = 0.0f64;
            for el in els {
                assert!(el.y >= prev_bottom - 0.01, "slide '{first}' elements overlap at y={}", el.y);
                prev_bottom = el.y + el.height.max(0.0);
            }
            assert!(prev_bottom <= CANVAS_H + 1.0, "slide '{first}' content exceeds 1080 canvas: bottom={prev_bottom}");
        }
    }
}
