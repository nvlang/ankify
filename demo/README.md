# Ankify demo

[`lecture.typ`](lecture.typ) is a short real-analysis lecture that doubles as an
Anki deck. Open it as a Typst document and it is a tidy set of notes; run the
`ankify` CLI on it and every definition and theorem becomes a flashcard.

It is the **helper pattern** in miniature: a `definition` and a `theorem` helper
each typeset an item *and* register a card, so the notes and the deck are
written in one place and can never drift apart.

## What it shows

- **The helper pattern** — `definition()` and `theorem()` helpers that typeset
  an item *and* register a card in a single call.
- **`configure()`** — document-wide defaults (deck, tags, render format,
  `scale`), and that it can be called more than once to adjust them.
- **`note()`** and its shorthands **`basic()`** and **`cloze()`**.
- **Render formats** — theme-aware `svg` cards, so the mathematics survives and
  follows Anki's light/dark mode, plus `plain` text for the quick-review cards.
- **Per-note decks and tags** — the theorems are routed to a
  `Real Analysis::Theorems` sub-deck with an extra tag.

## Running the demo

You need four things; all commands are run from the **repository root**.

1. **Anki**, running, with the
   [AnkiConnect](https://foosoft.net/projects/anki-connect/) add-on installed.

2. **The Typst CLI** (`typst`) on your `PATH` — see [typst.app](https://typst.app).

3. **The `ankify` CLI:**

   ```sh
   cargo install --path packages/ankify-cli
   ```

4. **The `ankify` Typst package**, installed locally so that
   `@preview/ankify:0.1.0` resolves:

   ```sh
   # macOS
   DEST="$HOME/Library/Application Support/typst/packages/preview/ankify/0.1.0"
   # Linux:   ~/.local/share/typst/packages/preview/ankify/0.1.0
   # Windows: %APPDATA%\typst\packages\preview\ankify\0.1.0

   mkdir -p "$DEST" && cp -r packages/ankify-typst/* "$DEST"
   ```

Then, with Anki open:

```sh
ankify demo/lecture.typ
```

## What you get

`ankify` creates two decks — **Real Analysis** and **Real Analysis::Theorems** —
and adds **7 notes**: three definitions, two theorems, and two quick-review
cards. Open Anki to study them.

## Experiment

`ankify` is incremental: a cache (`demo/.ankify/cache.json`) records what was
synced, so re-running only touches what changed.

- **Re-run** `ankify demo/lecture.typ` — nothing changed, so every note is
  skipped.
- **Edit** a definition in `lecture.typ`, then re-run — only that one card is
  updated in Anki.
- **Rename** a note's label (say `def-boundedness` → `def-bdd`) *without*
  changing its content, then re-run — `ankify` recognises the rename and updates
  the existing card in place rather than creating a duplicate.

## See it as a document

`lecture.typ` is also an ordinary Typst file. To produce just the PDF, with no
Anki involved:

```sh
typst compile demo/lecture.typ
```

---

New to Ankify? The [project README](../README.md) walks through the whole
workflow end to end.
