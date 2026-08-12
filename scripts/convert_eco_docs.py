#!/usr/bin/env python3
"""Convert eco_docs markdown into SlideDeck documents for the slides LXS.

Reads every *.md under the eco_docs VitePress frontend (excluding node_modules
and dist), chunks each page into a deck (one per H1, one slide per H2/H3
section, plus a title slide), lays elements out on the 1920x1080 canvas with
styles/effects/images, and emits one JSON document per deck.

The output shape matches the slides domain's SlideDeck model so the docs can
be seeded straight into the slides-backend Mongo collection `slide_decks`.
"""

import html
import json
import os
import re
import sys
import unicodedata

CW = 1920
CH = 1080
MARGIN = 110
CONTENT_W = CW - 2 * MARGIN  # 1700
TITLE_H = 150
BODY_Y0 = TITLE_H + 60
FONT = 34
LINE_H = FONT + 16
CODE_FONT = 26
CODE_LINE_H = CODE_FONT + 12

# Map a deck's doc category to a theme + accent so decks look varied.
THEME_BY_SECTION = {
    "guide": "light",
    "why": "vibrant",
    "concepts": "dark",
    "reference": "minimal",
    "case-study": "dark",
    "ecosphere": "vibrant",
    "tips": "light",
    "index": "vibrant",
    "contact": "minimal",
}
DEFAULT_THEME = "dark"

THEMES = {
    "light": {"bg": "#ffffff", "text": "#111827", "accent": "#3b82f6", "muted": "#6b7280", "code_bg": "#f3f4f6", "code_border": "#e5e7eb"},
    "dark": {"bg": "#121212", "text": "#ffffff", "accent": "#60a5fa", "muted": "#9ca3af", "code_bg": "#1f2937", "code_border": "#374151"},
    "vibrant": {"bg": "linear-gradient(135deg,#4f46e5 0%,#7c3aed 100%)", "text": "#ffffff", "accent": "#fbbf24", "muted": "#e0d7ff", "code_bg": "rgba(0,0,0,.28)", "code_border": "rgba(255,255,255,.28)"},
    "minimal": {"bg": "#f3f4f6", "text": "#1f2937", "accent": "#10b981", "muted": "#6b7280", "code_bg": "#e5e7eb", "code_border": "#d1d5db"},
}

SLUG_STOP = re.compile(r"[^a-z0-9-]")


def slugify(s: str) -> str:
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode()
    s = s.lower()
    s = SLUG_STOP.sub("-", s)
    s = re.sub(r"-{2,}", "-", s).strip("-")
    return s


def inline_to_text(line: str) -> str:
    """Strip inline markdown to plain text for a text element."""
    line = re.sub(r"`([^`]*)`", r"\1", line)
    line = re.sub(r"\*\*([^*]+)\*\*", r"\1", line)
    line = re.sub(r"\*([^*]+)\*", r"\1", line)
    line = re.sub(r"__([^_]+)__", r"\1", line)
    line = re.sub(r"_([^_]+)_", r"\1", line)
    line = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", line)
    line = re.sub(r"!\[([^\]]*)\]\([^)]*\)", r"\1", line)
    return line


def split_inline_tokens(line: str):
    """Return list of (text, bold) tokens so the player can apply bold."""
    tokens = []
    pos = 0
    pat = re.compile(r"(\*\*[^*]+\*\*|`[^`]*`|\*[^*]+\*)")
    for m in pat.finditer(line):
        if m.start() > pos:
            tokens.append((line[pos:m.start()], False))
        tok = m.group(0)
        bold = tok.startswith("**")
        tokens.append((tok.strip("*`"), bold))
        pos = m.end()
    if pos < len(line):
        tokens.append((line[pos:], False))
    return tokens


def strip_inline_md(line: str) -> str:
    return inline_to_text(line)


def normalize_blank(line: str) -> str:
    return line


def parse_markdown(text: str):
    """Split a markdown doc into an ordered list of blocks.

    Each block: {type, level?, title?, lines: [..] | code: [..] | rows: [[..]]}
    """
    blocks = []
    lines = text.splitlines()
    i = 0
    n = len(lines)
    pending = []  # paragraph / list buffer

    def flush_pending():
        nonlocal pending
        if pending:
            blocks.append({"type": "para", "lines": pending})
            pending = []

    while i < n:
        raw = lines[i]
        line = raw.rstrip()
        stripped = line.strip()

        if not stripped:
            flush_pending()
            i += 1
            continue

        # frontmatter
        if line.startswith("---") and i == 0:
            i += 1
            while i < n and not lines[i].startswith("---"):
                i += 1
            i += 1
            continue

        # heading
        m = re.match(r"^(#{1,4})\s+(.*)$", stripped)
        if m:
            flush_pending()
            level = len(m.group(1))
            blocks.append({"type": "heading", "level": level, "title": strip_inline_md(m.group(2))})
            i += 1
            continue

        # code fence (incl. mermaid — rendered as code)
        if stripped.startswith("```"):
            flush_pending()
            lang = stripped[3:].strip()
            i += 1
            code = []
            while i < n and not lines[i].strip().startswith("```"):
                code.append(lines[i])
                i += 1
            i += 1  # closing fence
            blocks.append({"type": "code", "lang": lang, "code": code})
            continue

        # table
        if stripped.startswith("|"):
            rows = []
            while i < n and lines[i].strip().startswith("|"):
                cells = [c.strip() for c in lines[i].strip().strip("|").split("|")]
                rows.append(cells)
                i += 1
            # drop separator row (|---|)
            rows = [r for r in rows if not all(re.fullmatch(r":?-{2,}:?", c or "-") for c in r)]
            flush_pending()
            blocks.append({"type": "table", "rows": rows})
            continue

        # horizontal rule
        if re.fullmatch(r"-{3,}|\*{3,}|_{3,}", stripped):
            flush_pending()
            i += 1
            continue

        # blockquote
        if stripped.startswith(">"):
            quote_lines = []
            while i < n and lines[i].strip().startswith(">"):
                q = lines[i].strip().lstrip(">").strip()
                if q:
                    quote_lines.append(strip_inline_md(q))
                i += 1
            flush_pending()
            blocks.append({"type": "quote", "lines": quote_lines})
            continue

        # list item(s)
        if re.match(r"^[-*+]\s+", stripped) or re.match(r"^\d+[.)]\s+", stripped):
            items = []
            while i < n and (re.match(r"^[-*+]\s+", lines[i].strip()) or re.match(r"^\d+[.)]\s+", lines[i].strip())):
                item = re.sub(r"^[-*+]\s+|^\d+[.)]\s+", "", lines[i].strip())
                items.append(strip_inline_md(item))
                i += 1
            flush_pending()
            blocks.append({"type": "list", "items": items})
            continue

        pending.append(strip_inline_md(stripped))
        i += 1

    flush_pending()
    return blocks


def estimate_text_height(text: str, width: int, font_size: int) -> int:
    """Approximate wrapped height for a text element."""
    cpl = max(int(width / (font_size * 0.55)), 8)
    n = 0
    for para in text.split("\n"):
        if not para:
            n += 1
            continue
        n += max(1, (len(para) + cpl - 1) // cpl)
    return n * (font_size + 12)


def split_long_text(text: str, width: int, font_size: int, max_h: int) -> list:
    """Split a body of text into chunks that each fit max_h, by paragraphs."""
    paras = text.split("\n")
    chunks = []
    cur = []
    cur_h = 0
    line_h = font_size + 12
    for p in paras:
        h = estimate_text_height(p, width, font_size)
        if cur and cur_h + h > max_h:
            chunks.append("\n".join(cur))
            cur = []
            cur_h = 0
        cur.append(p)
        cur_h += h
    if cur:
        chunks.append("\n".join(cur))
    return chunks


def add_element(slide, eid, etype, x, y, w, h, content, style=None, label=None, link_url=None):
    el = {
        "id": eid,
        "type": etype,
        "x": round(x, 1),
        "y": round(y, 1),
        "width": round(w, 1),
        "height": round(h, 1),
        "content": content or "",
    }
    if style:
        el["style"] = json.dumps(style)
    if label:
        el["label"] = label
    if link_url:
        el["linkUrl"] = link_url
    slide["elements"].append(el)


def build_slide(eid_prefix: str, name: str, level: int, slide_seq) -> dict:
    slide_seq[0] += 1
    return {"id": f"{eid_prefix}-s{slide_seq[0]}", "name": name, "level": level, "elements": []}


def layout_section_slides(prefix, heading_title, blocks, theme, eid_seq, eid_gen, slide_seq):
    """Turn a heading + its blocks into one or more slides."""
    t = THEMES[theme]
    slides = []
    body = []
    # Track whether we need the heading as a slide title.
    title_el_style = {
        "fontSize": 72,
        "color": t["accent"],
        "fontWeight": 800,
        "textAlign": "left",
        "displayEffect": "fade-down",
    }

    # Build slide pieces: title + flow of content blocks.
    # Simple pager: one slide per heading, but long text overflows onto extra slides.
    def new_slide():
        s = build_slide(prefix, f"s{slide_seq[0]+1}", 1, slide_seq)
        s["background"] = t["bg"]
        add_element(s, eid_gen(), "text", MARGIN, 70, CONTENT_W, 100, heading_title, title_el_style)
        return s

    current = new_slide()
    y = BODY_Y0
    block_idx = 0

    def fits(h, pad=20):
        return y + h <= CH - 90

    for blk in blocks:
        block_idx += 1
        if blk["type"] == "heading":
            continue  # nested headings already become section slides by caller

        if blk["type"] == "para":
            text = "\n".join(blk["lines"])
            if not text.strip():
                continue
            chunks = split_long_text(text, CONTENT_W, FONT, CH - BODY_Y0 - 100)
            for ci, chunk in enumerate(chunks):
                h = estimate_text_height(chunk, CONTENT_W, FONT)
                if not fits(h):
                    current = new_slide()
                    y = BODY_Y0
                style = {
                    "fontSize": FONT,
                    "color": t["text"],
                    "lineHeight": LINE_H,
                    "displayEffect": "fade",
                    "effectParams": {"delay": 150},
                }
                add_element(current, eid_gen(), "text", MARGIN, y, CONTENT_W, h, chunk, style)
                y += h + 26

        elif blk["type"] == "list":
            lines = blk["items"]
            text = "\n".join(f"•  {li}" for li in lines)
            chunks = split_long_text(text, CONTENT_W, FONT, CH - BODY_Y0 - 100)
            for chunk in chunks:
                h = estimate_text_height(chunk, CONTENT_W, FONT)
                if not fits(h):
                    current = new_slide()
                    y = BODY_Y0
                style = {
                    "fontSize": FONT,
                    "color": t["text"],
                    "lineHeight": LINE_H,
                    "displayEffect": "fade-left",
                    "effectParams": {"delay": 150},
                }
                add_element(current, eid_gen(), "text", MARGIN + 20, y, CONTENT_W - 20, h, chunk, style)
                y += h + 26

        elif blk["type"] == "quote":
            text = "“" + " ".join(blk["lines"]) + "”"
            h = estimate_text_height(text, CONTENT_W - 80, 40) + 40
            if not fits(h + 30):
                current = new_slide()
                y = BODY_Y0
            style = {
                "fontSize": 40,
                "color": t["accent"],
                "fontWeight": 700,
                "fontStyle": "italic",
                "borderLeft": f"6px solid {t['accent']}",
                "paddingLeft": 28,
                "displayEffect": "pop",
            }
            add_element(current, eid_gen(), "callout", MARGIN + 20, y, CONTENT_W - 40, h, text, style)
            y += h + 34

        elif blk["type"] == "code":
            code_lines = blk["code"]
            if not "".join(code_lines).strip():
                continue
            # max lines that fit one slide's body
            max_lines = max(int((CH - BODY_Y0 - 60) / CODE_LINE_H) - 1, 3)
            per_chunk = max_lines
            for start in range(0, len(code_lines), per_chunk):
                chunk_lines = code_lines[start : start + per_chunk]
                code = "\n".join(chunk_lines)
                if not code.strip():
                    continue
                h = len(chunk_lines) * CODE_LINE_H + 40
                if not fits(h + 20):
                    current = new_slide()
                    y = BODY_Y0
                style = {
                    "fontSize": CODE_FONT,
                    "color": "#e2e8f0",
                    "fontFamily": "monospace",
                    "backgroundColor": t["code_bg"],
                    "border": f"1px solid {t['code_border']}",
                    "borderRadius": 16,
                    "padding": 20,
                    "whiteSpace": "pre",
                    "overflow": "hidden",
                    "displayEffect": "slide-up",
                }
                add_element(current, eid_gen(), "text", MARGIN, y, CONTENT_W, h, code, style)
                y += h + 30

        elif blk["type"] == "table":
            rows = blk["rows"]
            if not rows:
                continue
            # render table as a monospace block, one row per line
            widths = []
            for c in range(max(len(r) for r in rows)):
                widths.append(max(len(r[c]) if c < len(r) else 0 for r in rows))
            lines = []
            for r in rows:
                line = "  ".join((r[c] if c < len(r) else "").ljust(widths[c]) for c in range(len(widths))).rstrip()
                lines.append(line)
            max_lines = max(int((CH - BODY_Y0 - 60) / CODE_LINE_H) - 1, 3)
            for start in range(0, len(lines), max_lines):
                slice_lines = lines[start : start + max_lines]
                text = "\n".join(slice_lines)
                h = len(slice_lines) * CODE_LINE_H + 40
                if not fits(h + 20):
                    current = new_slide()
                    y = BODY_Y0
                style = {
                    "fontSize": 26,
                    "color": t["text"],
                    "fontFamily": "monospace",
                    "backgroundColor": t["code_bg"],
                    "border": f"1px solid {t['code_border']}",
                    "borderRadius": 16,
                    "padding": 22,
                    "whiteSpace": "pre",
                    "overflow": "hidden",
                    "displayEffect": "fade",
                }
                add_element(current, eid_gen(), "text", MARGIN, y, CONTENT_W, h, text, style)
                y += h + 30

    slides.append(current)
    return slides


def build_deck(file_path: str, rel_path: str) -> dict:
    with open(file_path, encoding="utf-8") as f:
        text = f.read()

    section = rel_path.split("/")[0]
    theme = THEME_BY_SECTION.get(section, DEFAULT_THEME)
    t = THEMES[theme]

    blocks = parse_markdown(text)

    # Title from first H1, or filename.
    title = ""
    body_blocks = []
    for b in blocks:
        if b["type"] == "heading" and b["level"] == 1 and not title:
            title = b["title"]
        elif b["type"] == "heading" and b["level"] == 1:
            continue
        else:
            body_blocks.append(b)

    if not title:
        title = os.path.splitext(os.path.basename(rel_path))[0].replace("-", " ").title()

    slug = slugify(title) or slugify(rel_path)
    # Keep slug stable per doc
    slug = slugify(rel_path)[:60]

    subtitle = ""
    # first paragraph becomes the subtitle if it's short
    for b in body_blocks:
        if b["type"] == "para" and len(" ".join(b["lines"])) <= 220:
            subtitle = " ".join(b["lines"])
            break

    eid_seq = [0]

    def eid_gen():
        eid_seq[0] += 1
        return f"el{eid_seq[0]}"

    slide_seq = [0]
    slides = []

    # Title slide
    title_slide = build_slide(slug, "title", 0, slide_seq)
    title_slide["background"] = t["bg"]
    # big brand
    add_element(
        title_slide, eid_gen(), "text", MARGIN, 300, CONTENT_W, 140, title,
        {"fontSize": 96, "color": t["text"], "fontWeight": 900, "letterSpacing": "-0.02em", "textAlign": "left", "displayEffect": "fade-down"},
    )
    if subtitle:
        add_element(
            title_slide, eid_gen(), "text", MARGIN, 480, CONTENT_W, 90, subtitle,
            {"fontSize": 44, "color": t["accent"], "fontWeight": 600, "displayEffect": "fade-up", "effectParams": {"delay": 200}},
        )
    # logo image (eco docs brand mark), top-right
    add_element(
        title_slide, eid_gen(), "image", CW - 160 - MARGIN, 70, 160, 160,
        "https://eco.stuff8.com/logo-light.svg",
        {"displayEffect": "fade", "effectParams": {"delay": 400}},
    )
    add_element(
        title_slide, eid_gen(), "text", MARGIN, CH - 160, CONTENT_W, 60,
        f"ECO DOCS  ·  {section.upper()}",
        {"fontSize": 24, "color": t["muted"], "fontFamily": "monospace", "letterSpacing": "0.18em", "displayEffect": "fade", "effectParams": {"delay": 500}},
    )
    slides.append(title_slide)

    # Section slides: iterate blocks, opening a new slide at each H2/H3.
    current_heading = title
    section_blocks = []
    for b in body_blocks:
        if b["type"] == "heading" and b["level"] >= 2:
            if section_blocks:
                slides.extend(layout_section_slides(slug, current_heading, section_blocks, theme, eid_seq, eid_gen, slide_seq))
            current_heading = b["title"]
            section_blocks = []
        else:
            section_blocks.append(b)
    if section_blocks:
        slides.extend(layout_section_slides(slug, current_heading, section_blocks, theme, eid_seq, eid_gen, slide_seq))

    # Fallback: if no section slides were made (single chunk doc), ensure at least body.
    if len(slides) <= 1:
        slides.extend(layout_section_slides(slug, title, body_blocks, theme, eid_seq, eid_gen, slide_seq))

    return {
        "name": title,
        "slug": slug,
        "subtitle": subtitle,
        "description": subtitle or title,
        "tags": [section, "eco-docs", "docs"],
        "level": "Intermediate",
        "language": "en",
        "instructorName": "Eco Docs",
        "status": "published",
        "ownerId": "getecosphere",
        "theme": theme,
        "layoutFormat": "presentation",
        "paywallStartSlideIndex": 99999,
        "slides": slides,
        "createdAt": "2026-08-12T00:00:00Z",
        "updatedAt": "2026-08-12T00:00:00Z",
    }


def main():
    docs_root = sys.argv[1] if len(sys.argv) > 1 else "."
    out_dir = sys.argv[2] if len(sys.argv) > 2 else "decks"
    os.makedirs(out_dir, exist_ok=True)

    total_decks = 0
    total_slides = 0
    for root, dirs, files in os.walk(docs_root):
        dirs[:] = [d for d in dirs if d not in ("node_modules", "dist", ".git")]
        for fn in sorted(files):
            if not fn.endswith(".md"):
                continue
            fp = os.path.join(root, fn)
            rel = os.path.relpath(fp, docs_root)
            deck = build_deck(fp, rel)
            out = os.path.join(out_dir, deck["slug"] + ".json")
            with open(out, "w", encoding="utf-8") as f:
                json.dump(deck, f, ensure_ascii=False, indent=1)
            total_decks += 1
            total_slides += len(deck["slides"])
            print(f"{deck['slug']:48s} {len(deck['slides']):3d} slides  {rel}")
    print(f"\n{total_decks} decks, {total_slides} slides -> {out_dir}/")


if __name__ == "__main__":
    main()
