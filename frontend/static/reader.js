// ecobook reader — Phaser.js portrait fullscreen.
// Pixel-perfect portrait pages: paper background, Times New Roman, normal size.
// Features:
//   * fade in/out page transition
//   * 3D page-turn (mesh rotationY around the spine) with a subtle corner skew
//   * long-press (or tap the bottom edge) toggles a bottom control bar:
//       - page number (display + input to jump)
//       - A / a font size (enlarge / shrink)
//       - deck / black / paper / white theme buttons (deck = authored palette
//         from the deck's baked theme tokens; falls back to paper)
//       - first / prev / next / last navigation
//       - exit button (window.history.back())
//   * long-press again closes the bar
//   * no toolbar otherwise; exit via browser back button.

(function () {
  var SLUG = window.__SLUG__ || (function () {
    var m = location.pathname.match(/\/ecobook\/([^/]+)/);
    return m ? decodeURIComponent(m[1]) : "";
  })();

  var API = (location.hostname === "127.0.0.1" || location.hostname === "localhost")
    ? "http://127.0.0.1:9015/api/book/"
    : "/api/book/";

  // themes: paper / white / black — plus a deck-defined theme applied from
  // the deck's baked token blob (see ecobook deck-document format). The deck
  // theme is derived from getecosphere design tokens (bg/ink/accent/surface).
  var THEMES = {
    paper: { bg: 0xf4ead6, ink: 0x4a3b28, accent: 0x9a6b1f, barBg: 0xe8d9ba },
    white: { bg: 0xffffff, ink: 0x222222, accent: 0x1f6feb, barBg: 0xf0f0f0 },
    black: { bg: 0x141414, ink: 0xe8e6e3, accent: 0xd9b64a, barBg: 0x222222 }
  };
  var FONT = "'Times New Roman', Georgia, serif";

  function parseColor(c) {
    if (typeof c !== "string") return null;
    var m = c.trim().match(/^#([0-9a-fA-F]{6})$/);
    return m ? parseInt(m[1], 16) : null;
  }

  // Merge the deck's baked theme tokens (JSON blob in deck.theme) into THEMES
  // so the imported deck renders with its authored palette by default.
  function applyDeckTheme(deck) {
    var raw = deck && deck.theme;
    if (!raw) return;
    var t = null;
    try { t = typeof raw === "string" ? JSON.parse(raw) : raw; } catch (e) { return; }
    var bg = parseColor(t.bg), ink = parseColor(t.ink), accent = parseColor(t.accent);
    if (bg === null && ink === null && accent === null) return;
    THEMES.deck = {
      bg: bg !== null ? bg : 0xf7f6f2,
      ink: ink !== null ? ink : 0x17141d,
      accent: accent !== null ? accent : 0x5b3fd6,
      barBg: bg !== null ? bg : 0xf7f6f2
    };
    themeName = "deck";
    if (typeof t.fontDisplay === "string" && t.fontDisplay.length) {
      FONT = "'" + t.fontDisplay.split(",")[0].trim().replace(/^['"]|['"]$/g, "") + "', Georgia, serif";
    }
  }

  var PW = 900, PH = 1440;
  var pages = [], pageIdx = 0;
  var fontSize = 34;              // normal size
  var themeName = "paper";
  var turning = false;

  function styleJson(el) {
    try { return JSON.parse(el.style || "{}"); } catch (e) { return {}; }
  }

  function buildPages(deck) {
    pages = [];
    pages.push({ title: deck.name || "", subtitle: deck.subtitle || "", blocks: [] });
    (deck.slides || []).forEach(function (slide) {
      if (slide.level === 0) return;
      var blocks = [];
      (slide.elements || []).forEach(function (el) {
        var st = styleJson(el); var c = el.content || "";
        if (!c.trim()) return;
        if (el.type === "callout") blocks.push({ kind: "callout", text: c });
        else if (st.whiteSpace === "pre" || st.fontFamily === "monospace" || st.backgroundColor) blocks.push({ kind: "code", text: c });
        else if (el.type === "text" || el.type === "paragraph") blocks.push({ kind: "para", text: c });
      });
      if (!blocks.length) return;
      var first = (slide.elements || []).find(function (el) { return el.type === "text" || el.type === "paragraph"; });
      pages.push({ title: (first && first.content) ? first.content : (slide.name || "Page " + pages.length), blocks: blocks });
    });
  }

  var scene = new Phaser.Class({
    Extends: Phaser.Scene,
    initialize: function BookScene () { Phaser.Scene.call(this, { key: "book" }); },

    create: function () {
      var t = THEMES[themeName];
      this.cameras.main.setBackgroundColor(t.bg);
      this.paperBg = this.add.rectangle(PW / 2, PH / 2, PW, PH, t.bg);

      // blank white texture used for the turning paper leaf
      var g0 = this.add.graphics();
      g0.fillStyle(0xffffff, 1); g0.fillRect(0, 0, 1, 1);
      g0.generateTexture("__BLANK", 1, 1);
      g0.destroy();

      this.titleText = this.add.text(90, 120, "", { fontFamily: FONT, fontSize: 44, color: this.hex(t.accent), fontStyle: "bold", wordWrap: { width: PW - 180 } });
      this.bodyText = this.add.text(90, 210, "", { fontFamily: FONT, fontSize: fontSize, color: this.hex(t.ink), lineSpacing: Math.round(fontSize * 0.35), wordWrap: { width: PW - 180 }, align: "justify" });
      this.pagenoText = this.add.text(PW - 90, PH - 90, "", { fontFamily: FONT, fontSize: 24, color: this.hex(t.accent) }).setOrigin(1, 1);

      // tap zones (only when bar is closed)
      this.prevZone = this.add.zone(0, PH / 2, PW / 3, PH).setOrigin(0, 0.5).setInteractive();
      this.nextZone = this.add.zone(PW / 3, PH / 2, (PW * 2) / 3, PH).setOrigin(0, 0.5).setInteractive();

      var self = this;
      this.input.keyboard.on("keydown-LEFT", function () { self.turn(-1); });
      this.input.keyboard.on("keydown-RIGHT", function () { self.turn(1); });
      this.input.keyboard.on("keydown-SPACE", function () { self.turn(1); });

      // long-press detection on the whole game surface
      this.pressTimer = null;
      this.input.on("pointerdown", function (p) {
        // ignore presses that land on the bar's zones (handled separately below)
        if (self.bar && self.bar.visible) return;
        self.pressTimer = self.time.delayedCall(550, function () { self.toggleBar(); });
      });
      this.input.on("pointerup", function (p) {
        if (self.pressTimer) { self.pressTimer.remove(); self.pressTimer = null; }
      });

      this.prevZone.on("pointerdown", function () {
        if (self.barOpen) return;
        self.turn(-1);
      });
      this.nextZone.on("pointerdown", function () {
        if (self.barOpen) return;
        self.turn(1);
      });

      this.buildBar();
      this.renderPage(true);
    },

    hex: function (n) { return "#" + n.toString(16).padStart(6, "0"); },

    // build the bottom control bar (hidden by default)
    buildBar: function () {
      var self = this;
      var t = THEMES[themeName];
      this.barOpen = false;

      this.bar = this.add.container(0, 0).setDepth(10);
      this.barBg = this.add.rectangle(PW / 2, PH - 110, PW, 220, t.barBg).setStrokeStyle(2, 0x000000, 0.08);
      this.barTitle = this.add.text(30, PH - 205, "", { fontFamily: FONT, fontSize: 22, color: this.hex(t.ink), fontStyle: "bold" }).setOrigin(0, 0);
      this.barPage = this.add.text(PW - 30, PH - 205, "", { fontFamily: FONT, fontSize: 22, color: this.hex(t.accent) }).setOrigin(1, 0);
      this.bar.add([this.barBg, this.barTitle, this.barPage]);

      // font size buttons: a (shrink) and A (enlarge)
      this.btnA = this.makeBarBtn(PW / 2 - 210, PH - 150, "A", function () { self.setFontSize(fontSize + 4); });
      this.btnSmallA = this.makeBarBtn(PW / 2 - 160, PH - 150, "a", function () { self.setFontSize(fontSize - 4); });

      // theme buttons: deck (authored palette) / black / paper / white
      this.btnDeck = this.makeBarBtn(PW / 2 - 140, PH - 150, "✎", function () { self.setTheme("deck"); });
      this.btnBlack = this.makeBarBtn(PW / 2 - 90, PH - 150, "◼", function () { self.setTheme("black"); });
      this.btnPaper = this.makeBarBtn(PW / 2 - 40, PH - 150, "📄", function () { self.setTheme("paper"); });
      this.btnWhite = this.makeBarBtn(PW / 2 + 10, PH - 150, "◻", function () { self.setTheme("white"); });

      // navigation: first | prev | next | last
      this.btnFirst = this.makeBarBtn(30, PH - 150, "⏮", function () { self.turnTo(0); });
      this.btnPrev = this.makeBarBtn(80, PH - 150, "◀", function () { self.turn(-1); });
      this.btnNext = this.makeBarBtn(130, PH - 150, "▶", function () { self.turn(1); });
      this.btnLast = this.makeBarBtn(180, PH - 150, "⏭", function () { self.turnTo(pages.length - 1); });

      // page number input
      this.pageInput = this.add.rectangle(PW - 250, PH - 150, 120, 40, 0xffffff, 0.9).setStrokeStyle(1, 0x000000, 0.2);
      this.pageInputText = this.add.text(PW - 250, PH - 150, String(pageIdx + 1), { fontFamily: FONT, fontSize: 22, color: "#222222" }).setOrigin(0.5);
      this.pageInput.setInteractive();
      this.pageInput.on("pointerdown", function () { self.focusInput(); });

      // exit button
      this.btnExit = this.makeBarBtn(PW - 40, PH - 150, "✕", function () { if (window.history.length > 1) window.history.back(); else window.location.href = "/"; });

      this.bar.add([this.btnA, this.btnSmallA, this.btnDeck, this.btnBlack, this.btnPaper, this.btnWhite,
                    this.btnFirst, this.btnPrev, this.btnNext, this.btnLast,
                    this.pageInput, this.pageInputText, this.btnExit]);

      // keyboard: type digits into the page input when focused
      this.input.keyboard.on("keydown", function (e) {
        if (!self.inputFocused) return;
        if (e.key >= "0" && e.key <= "9") {
          self.inputBuf = (self.inputBuf || "") + e.key;
          self.pageInputText.setText(self.inputBuf);
        } else if (e.key === "Enter") {
          self.goToInput();
        } else if (e.key === "Backspace") {
          self.inputBuf = (self.inputBuf || "").slice(0, -1);
          self.pageInputText.setText(self.inputBuf || String(pageIdx + 1));
        }
      });

      this.bar.setVisible(false);
    },

    makeBarBtn: function (x, y, label, cb) {
      var btn = this.add.rectangle(x, y, 46, 46, 0xffffff, 0.85).setStrokeStyle(1, 0x000000, 0.18).setInteractive({ useHandCursor: true });
      var txt = this.add.text(x, y, label, { fontFamily: FONT, fontSize: 22, color: "#222222" }).setOrigin(0.5);
      btn.on("pointerdown", cb);
      return this.add.container(0, 0, [btn, txt]).setPosition(0, 0);
    },

    toggleBar: function () {
      this.barOpen = !this.barOpen;
      this.bar.setVisible(this.barOpen);
      if (this.barOpen) {
        this.inputFocused = false; this.inputBuf = "";
        var t = THEMES[themeName];
        this.barTitle.setText(pages[pageIdx] ? pages[pageIdx].title : "");
        this.barPage.setText((pageIdx + 1) + " / " + pages.length);
        this.pageInputText.setText(String(pageIdx + 1));
        this.barBg.setFillStyle(t.barBg);
      } else {
        this.inputFocused = false;
      }
    },

    focusInput: function () {
      this.inputFocused = true; this.inputBuf = "";
      this.pageInputText.setText("");
    },

    goToInput: function () {
      var n = parseInt(this.inputBuf, 10);
      this.inputFocused = false;
      if (isNaN(n)) { this.pageInputText.setText(String(pageIdx + 1)); return; }
      this.turnTo(n - 1);
    },

    setFontSize: function (size) {
      fontSize = Math.max(22, Math.min(54, size));
      this.bodyText.setFontSize(fontSize).setLineSpacing(Math.round(fontSize * 0.35));
    },

    setTheme: function (name) {
      themeName = name;
      var t = THEMES[name];
      this.cameras.main.setBackgroundColor(t.bg);
      this.paperBg.setFillStyle(t.bg);
      this.titleText.setColor(this.hex(t.accent));
      this.bodyText.setColor(this.hex(t.ink));
      this.pagenoText.setColor(this.hex(t.accent));
      this.barBg.setFillStyle(t.barBg);
      this.barTitle.setColor(this.hex(t.ink));
      this.barPage.setColor(this.hex(t.accent));
      this.renderPage(true);
    },

    // fade + 3D page-turn
    turn: function (delta) {
      var next = pageIdx + delta;
      if (turning || next < 0 || next >= pages.length) return;
      this.turnTo(next);
    },

    turnTo: function (target) {
      if (turning) return;
      target = Math.max(0, Math.min(pages.length - 1, target));
      if (target === pageIdx) { this.renderPage(true); return; }
      var forward = target > pageIdx;
      var self = this;
      var fromIdx = pageIdx;
      var toIdx = target;
      turning = true;

      // 1) fade out the current page
      this.cameras.main.fadeOut(180, 0, 0, 0);
      this.cameras.main.once("camerafadeoutcomplete", function () {
        // 2) swap to the target page underneath
        pageIdx = toIdx;
        self.renderPage(false);

        // 3) animate a paper leaf turning around the spine (left edge), with a
        //    slight corner skew via perspective, revealing the next page
        var t = THEMES[themeName];
        var vw = PW, vh = PH;
        var verts = [0,0,0, vw,0,0, 0,vh,0, vw,vh,0];
        var uvs = [0,0, 1,0, 0,1, 1,1];
        var idx = [0,1,2, 1,3,2];
        var mesh = self.add.mesh(0, 0, verts, uvs, idx, true, undefined, undefined, undefined, "__BLANK");
        mesh.setPerspective(Math.max(PW, PH) * 1.6);
        mesh.setPosition(0, 0);
        mesh.setDepth(5);
        // tint the blank leaf to the paper color so it reads as a turning page
        mesh.setTint(t.bg);
        var endRot = forward ? -Math.PI : Math.PI;
        self.tweens.add({
          targets: mesh, rotationY: endRot, duration: 800, ease: "Cubic.easeInOut",
          onComplete: function () {
            mesh.destroy();
            turning = false;
            self.cameras.main.resetFX();
            self.cameras.main.fadeIn(180, 0, 0, 0);
          }
        });
      });
    },

    renderPage: function (instant) {
      var page = pages[pageIdx];
      if (!page) return;
      var t = THEMES[themeName];
      this.paperBg.setFillStyle(t.bg);
      if (pageIdx === 0) {
        this.titleText.setFontSize(56).setOrigin(0.5, 0).setPosition(PW / 2, 500).setAlign("center").setText(page.title || "");
        this.bodyText.setFontSize(38).setOrigin(0.5, 0).setPosition(PW / 2, 640).setAlign("center").setText(page.subtitle || "");
        this.pagenoText.setText("");
        return;
      }
      this.titleText.setOrigin(0, 0).setPosition(90, 110).setAlign("left").setFontSize(44).setText(page.title || "");
      var body = page.blocks.map(function (b) { return b.kind === "callout" ? "❝ " + b.text + " ❞" : b.text; }).join("\n\n");
      this.bodyText.setOrigin(0, 0).setPosition(90, 210).setAlign("justify").setFontSize(fontSize).setLineSpacing(Math.round(fontSize * 0.35)).setText(body);
      this.pagenoText.setText((pageIdx + 1) + " / " + pages.length);
      if (instant) { this.cameras.main.resetFX(); this.cameras.main.fadeIn(200, 0, 0, 0); }
    }
  });

  fetch(API + encodeURIComponent(SLUG))
    .then(function (r) { return r.ok ? r.json() : null; })
    .then(function (deck) {
      if (!deck) { document.title = "Ecobook — not found"; return; }
      document.title = deck.name + " — Ecobook";
      applyDeckTheme(deck);
      buildPages(deck);
      var t0 = THEMES[themeName] || THEMES.paper;
      var config = {
        type: Phaser.AUTO,
        width: PW, height: PH, backgroundColor: "#" + t0.bg.toString(16).padStart(6, "0"),
        scale: { mode: Phaser.Scale.FIT, autoCenter: Phaser.Scale.CENTER_BOTH, width: PW, height: PH },
        parent: "app",
        scene: [scene]
      };
      new Phaser.Game(config);
    })
    .catch(function () { document.title = "Ecobook — could not load"; });
})();
