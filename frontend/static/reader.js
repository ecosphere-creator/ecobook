// ecobook reader — Phaser.js portrait fullscreen.
// Pixel-perfect portrait pages: paper background, Times New Roman, normal
// size. Next/prev page (arrow keys / Space / tap edges). No toolbar, no
// page-turn animation. Exit by pressing the browser back button.

(function () {
  var SLUG = window.__SLUG__ || (function () {
    var m = location.pathname.match(/\/ecobook\/([^/]+)/);
    return m ? decodeURIComponent(m[1]) : "";
  })();

  var API = (location.hostname === "127.0.0.1" || location.hostname === "localhost")
    ? "http://127.0.0.1:9015/api/book/"
    : "/api/book/";

  var PAPER = 0xf4ead6;   // warm paper
  var INK = 0x4a3b28;     // warm brown ink
  var ACCENT = 0x9a6b1f;  // heading accent
  var FONT = "'Times New Roman', Georgia, serif";

  // portrait page size (fixed logical pixels; Phaser scales to fit screen)
  var PW = 900, PH = 1440;

  var pages = [];  // array of { title, blocks: [{kind, text}] }
  var pageIdx = 0;

  function styleJson(el) {
    try { return JSON.parse(el.style || "{}"); } catch (e) { return {}; }
  }

  // Build portrait pages from a deck: a title page + one page per slide,
  // each slide's text/callout/code elements as blocks.
  function buildPages(deck) {
    pages = [];
    // cover page
    pages.push({
      title: deck.name || "",
      subtitle: deck.subtitle || "",
      blocks: []
    });
    (deck.slides || []).forEach(function (slide) {
      if (slide.level === 0) return; // title slide already rendered as cover
      var blocks = [];
      (slide.elements || []).forEach(function (el) {
        var st = styleJson(el);
        var content = el.content || "";
        if (!content.trim()) return;
        if (el.type === "callout") blocks.push({ kind: "callout", text: content });
        else if (st.whiteSpace === "pre" || st.fontFamily === "monospace" || st.backgroundColor) {
          blocks.push({ kind: "code", text: content });
        } else if (el.type === "text" || el.type === "paragraph") {
          blocks.push({ kind: "para", text: content });
        }
      });
      // skip slides that contribute nothing
      if (!blocks.length) return;
      var first = (slide.elements || []).find(function (el) { return el.type === "text" || el.type === "paragraph"; });
      pages.push({
        title: (first && first.content) ? first.content : (slide.name || "Page " + pages.length),
        blocks: blocks
      });
    });
  }

  var scene = {
    preload: function () {
      // nothing to preload; fonts come from CSS/DOM default
    },
    create: function () {
      this.paperBg = this.add.rectangle(PW / 2, PH / 2, PW, PH, PAPER);
      this.titleText = this.add.text(90, 120, "", {
        fontFamily: FONT, fontSize: 44, color: "#9a6b1f", fontStyle: "bold",
        wordWrap: { width: PW - 180 }
      });
      this.bodyText = this.add.text(90, 210, "", {
        fontFamily: FONT, fontSize: 34, color: "#4a3b28", lineSpacing: 12,
        wordWrap: { width: PW - 180 }, align: "justify"
      });
      this.pagenoText = this.add.text(PW - 90, PH - 90, "", {
        fontFamily: FONT, fontSize: 26, color: "#9a6b1f"
      }).setOrigin(1, 1);

      // pointer/tap zones: left third = prev, right two thirds = next
      this.prevZone = this.add.zone(0, PH / 2, PW / 3, PH).setOrigin(0, 0.5).setInteractive();
      this.nextZone = this.add.zone(PW / 3, PH / 2, (PW * 2) / 3, PH).setOrigin(0, 0.5).setInteractive();
      this.prevZone.on("pointerdown", function () { this.go(-1); }, this);
      this.nextZone.on("pointerdown", function () { this.go(1); }, this);

      // keyboard
      var self = this;
      this.input.keyboard.on("keydown-LEFT", function () { self.go(-1); });
      this.input.keyboard.on("keydown-RIGHT", function () { self.go(1); });
      this.input.keyboard.on("keydown-SPACE", function () { self.go(1); });

      this.renderPage();
    },
    go: function (delta) {
      var next = pageIdx + delta;
      if (next >= 0 && next < pages.length) { pageIdx = next; this.renderPage(); }
    },
    renderPage: function () {
      var page = pages[pageIdx];
      if (!page) return;
      var blocks = page.blocks || [];
      // cover: title + subtitle centered
      if (pageIdx === 0) {
        this.titleText.setText(page.title || "").setFontSize(56).setOrigin(0.5, 0).setPosition(PW / 2, 500).setAlign("center");
        this.bodyText.setText(page.subtitle || "").setFontSize(38).setOrigin(0.5, 0).setPosition(PW / 2, 640).setAlign("center");
        this.pagenoText.setText("");
        return;
      }
      // content page: heading + flowed blocks
      this.titleText.setOrigin(0, 0).setPosition(90, 110).setAlign("left").setFontSize(44).setText(page.title || "");
      var body = blocks.map(function (b) {
        if (b.kind === "callout") return "❝ " + b.text + " ❞";
        if (b.kind === "code") return b.text;
        return b.text;
      }).join("\n\n");
      this.bodyText.setOrigin(0, 0).setPosition(90, 210).setAlign("justify").setFontSize(34).setText(body);
      this.pagenoText.setText((pageIdx + 1) + " / " + pages.length).setOrigin(1, 1).setPosition(PW - 90, PH - 90);
    }
  };

  fetch(API + encodeURIComponent(SLUG))
    .then(function (r) { return r.ok ? r.json() : null; })
    .then(function (deck) {
      if (!deck) { document.title = "Ecobook — not found"; return; }
      document.title = deck.name + " — Ecobook";
      buildPages(deck);
      var config = {
        type: Phaser.AUTO,
        width: PW,
        height: PH,
        backgroundColor: "#f4ead6",
        scale: {
          mode: Phaser.Scale.FIT,
          autoCenter: Phaser.Scale.CENTER_BOTH,
          width: PW,
          height: PH
        },
        parent: "app",
        scene: { preload: scene.preload, create: scene.create }
      };
      new Phaser.Game(config);
    })
    .catch(function () { document.title = "Ecobook — could not load"; });
})();
