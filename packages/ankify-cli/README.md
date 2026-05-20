<br>
<div align="center">
<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/nvlang/ankify/main/res/dark/logotype.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/nvlang/ankify/main/res/light/logotype.svg">
    <img alt="Ankify Logotype" src="https://raw.githubusercontent.com/nvlang/ankify/main/res/light/logotype.svg" width="33%">
</picture>
<br>
<br>
<div>

[<picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/ankify-_?style=flat-square&logo=typst&logoColor=a3acb7&labelColor=21262d&color=21262d&logoSize=auto"><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/badge/ankify-_?style=flat-square&logo=typst&logoColor=24292f&labelColor=eaeef2&color=eaeef2&logoSize=auto"><img alt="Typst package name" src="https://img.shields.io/badge/ankify-_?style=flat-square&logo=typst&logoColor=24292f&labelColor=eaeef2&color=eaeef2&logoSize=auto"></picture>](https://typst.app/universe/package/ankify)
[<picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/crates/ankify-_?style=flat-square&logo=rust&logoColor=a3acb7&labelColor=21262d&color=21262d&logoSize=auto"><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/badge/crates/ankify-_?style=flat-square&logo=rust&logoColor=24292f&labelColor=eaeef2&color=eaeef2&logoSize=auto"><img alt="Rust crate name" src="https://img.shields.io/badge/crates/ankify-_?style=flat-square&logo=rust&logoColor=24292f&labelColor=eaeef2&color=eaeef2&logoSize=auto"></picture>](https://crates.io/crates/ankify)
</div>
</div>
<br>

> [!WARNING]
> `ankify` is in alpha and under active development. Expect bugs, rough edges,
> and breaking changes between releases.

`ankify` is the command-line tool — and library — at the heart of
[Ankify](https://github.com/nvlang/ankify). It reads a Typst document that uses
the [`ankify` Typst package](https://typst.app/universe/package/ankify), renders
each flashcard, and syncs it to Anki through the
[AnkiConnect](https://foosoft.net/projects/anki-connect/) add-on — adding new
cards, updating changed ones, and leaving the rest untouched.

## Installation

```sh
cargo install ankify
```

To run it you also need:

- the [Typst](https://typst.app) CLI (`typst`) on your `PATH`;
- [Anki](https://apps.ankiweb.net), running, with the **AnkiConnect** add-on;
- the [`ankify` Typst package](https://typst.app/universe/package/ankify) — your
  documents import it to mark up cards.

## Usage

```sh
ankify notes.typ
```

```
ankify <FILE> [options]

  -v, --verbose                Verbose output
      --cache-file <PATH>      Custom cache file location
      --ankiconnect-url <URL>  AnkiConnect URL (default: http://127.0.0.1:8765)
      --root <DIR>             Typst project root
      --font-path <PATH>       Additional font path (repeatable)
```

Re-running `ankify` on the same file is incremental: a cache
(`.ankify/cache.json`, next to the document) records what was synced, so only
new or changed notes are sent to Anki.

The [project README](https://github.com/nvlang/ankify) covers the end-to-end
workflow — writing notes, the helper-function pattern, and theme-aware cards.

## Library

The same crate is also a library, should you want to drive a sync
programmatically:

```sh
cargo add ankify
```

```rust
use ankify::sync::{sync, SyncConfig};

let result = sync(SyncConfig::new("notes.typ")).await?;
println!("{} added, {} updated", result.notes_added, result.notes_updated);
```

API documentation is published on [docs.rs](https://docs.rs/ankify).

## License

MIT — see [LICENSE](LICENSE).

## Trademarks

Anki is a trademark of Ankitects Pty Ltd. `ankify` is an independent project,
not affiliated with, endorsed by, or sponsored by Ankitects Pty Ltd.
