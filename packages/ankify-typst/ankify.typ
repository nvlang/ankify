// See the file LICENSE for the full license governing this code.

#import "util/validate.typ": _validate-data

#let ankify-notes = state("ankify-notes", ())
#let ankify-configuration = state(
  "ankify-configuration",
  (
    ankiconnect-url: "http://localhost:8765",
    verbose: false,
    defaults: (
      model: "Basic",
      deck: "Default",
      format: "png",
    ),
    render: "ankify-render",
    cache: (
      enabled: true,
      custom-file: none, // use default
    ),
    checks: (
      typst: (
        data: true,
        format: true,
      ),
      ankiconnect: (
        model: true,
        deck: true,
        tags: true,
      ),
    ),
  ),
)

/// Ankify Typst Extension
///
/// This file provides functions for creating Anki cards and configuring
/// the Ankify CLI tool from within Typst documents.
///
/// ## Usage
///
/// ```typst
/// #import "ankify.typ": card, configure
///
/// // Configure Ankify settings
/// #configure(
///   ankiconnect-url: "http://localhost:8765",
///   verbose: true,
///   defaults: (
///     model: "Basic",
///     deck: "MyDeck",
///     tags: ("study", "typst")
///   )
/// )
///
/// // Create cards
/// #note(
///   "pythagorean-theorem",
///   model: "Basic",
///   data: (
///     Front: "What is the Pythagorean theorem?",
///     Back: "a² + b² = c²"
///   ),
///   deck: "Mathematics",
///   tags: ("geometry", "theorem"),
///   format: "svg"
/// )
/// ```


/// Create an Anki card with the specified metadata.
///
/// This function stores card data as metadata that can be extracted by the
/// Ankify CLI tool. It does not render the card content - that is handled
/// by the CLI tool based on the format and render settings.
///
/// === Arguments
///
/// - `label` (required): Unique identifier for this note
/// - `model` (optional): Anki note type (defaults to "Basic")
/// - `data` (required): Dictionary of field names to content
/// - `deck` (optional): Anki deck name (defaults to "Default")
/// - `tags` (optional): Array of tags to apply
/// - `other` (optional): Additional metadata to pass to AnkiConnect
/// - `format` (optional): Rendering format ("svg", "png", "plain")
///
/// === Examples
///
/// Basic note:
///
/// ```typst
/// #note(
///   "basic-example",
///   data: (
///     Front: "What is Rust?",
///     Back: "A systems programming language"
///   )
/// )
/// ```
///
/// Note with full options:
///
/// ```typst
/// #note(
///   "advanced-example",
///   model: "Basic (and reversed note)",
///   data: (
///     Front: "Question text",
///     Back: "Answer text",
///     Extra: "Additional information"
///   ),
///   deck: "Computer Science",
///   tags: ("programming", "rust"),
///   format: "png",
///   other: (
///     customField: "custom value"
///   )
/// )
/// ```
#let note(
  label: str,
  data: dictionary,
  model: str,
  deck: str,
  tags: (str,),
  other: dictionary,
  format: str,
) = {
  context {
    let checks = ankify-configuration.get().checks

    // Validate required arguments
    assert(type(label) == str and label != "", message: "Note/card label must be a non-empty string")

    if (checks.typst.data != false) {
      _validate-data(data: data)
    }

    assert(
      checks.typst.format == false or format == none or (type(format) == str and format in ("svg", "png", "plain")),
      message: "`format` must be one of: \"svg\", \"png\", \"plain\"",
    )

    // Store as metadata for CLI extraction
    [#metadata((
        label: label,
        data: data,
        model: model,
        tags: tags,
        deck: deck,
        other: other,
        format: format,
      )) <ankify-note>]

    // Return both the update (which places the state change) and the content
    [
      #ankify-notes.update(notes => {
        notes.push(x)
        notes
      })
    ]
  }
}

/// Configure Ankify settings for the current document.
///
/// This function stores configuration metadata that affects how the Ankify
/// CLI tool processes cards in this document. Settings specified here can
/// be overridden by CLI arguments.
///
/// === Arguments
///
/// - `ankiconnect-url` (optional): URL for AnkiConnect API
/// - `verbose` (optional): Enable verbose output
/// - `defaults` (optional): Default values for card fields
/// - `render` (optional): Name of custom render function
/// - `cache` (optional): Cache settings
///   - `enabled`: Whether to enable caching (default: true)
///   - `custom-file`: Path to custom cache file (default: none, uses default cache)
/// - `checks` (optional): Validation checks to perform
///   - `typst`: Checks for Typst data and format
///   - `ankiconnect`: Checks for AnkiConnect fields like model, deck, or tags
///
/// === Examples
///
/// Basic configuration:
//
/// ```typst
/// #configure(
///   ankiconnect-url: "http://localhost:8765",
///   verbose: true
/// )
/// ```
///
/// Configuration with example defaults:
///
/// ```typst
/// #configure(
///   defaults: (
///     model: "Basic",
///     deck: "MyStudyDeck",
///     tags: ("study", "important"),
///     data: (
///       Extra: "Default extra information"
///     )
///   )
/// )
/// ```
///
/// Configuration with different render function name:
///
/// ```typst
/// #configure(
///   render: "custom-render",
///   defaults: (
///     format: "png"
///   )
/// )
/// ```
#let configure(
  ankiconnect-url: "http://localhost:8765",
  verbose: false,
  defaults: (:),
  render: "ankify-render",
  cache: (
    enabled: true,
    custom-file: none, // use default
  ),
  checks: (
    typst: (
      data: true,
      format: true,
    ),
    ankiconnect: (
      model: true,
      deck: true,
      tags: true,
    ),
  ),
) = {
  // Type-check arguments
  assert(ankiconnect-url == none or type(ankiconnect-url) == str, message: "AnkiConnect URL must be a string")
  assert(verbose == none or type(verbose) == bool, message: "Verbose must be a boolean")
  assert(render == none or type(render) == str, message: "Render function name must be a string")
  if (cache != none) {
    assert(
      type(cache) == dictionary,
      message: "Cache configuration must be a dictionary",
    )
    for (key, value) in cache {
      if (key == "enabled") {
        assert(
          type(value) == bool,
          message: "Cache enabled must be a boolean",
        )
      } else if (key == "custom-file") {
        assert(
          type(value) == str or value == none,
          message: "Cache custom file must be a string or none",
        )
      } else {
        panic("Unknown cache key: " + key + ". Valid keys are: \"enabled\", \"custom-file\"")
      }
    }
  }

  if defaults != none {
    assert(
      type(defaults) == dictionary,
      message: "Defaults must be a dictionary",
    )

    // Validate defaults structure
    for (key, value) in defaults {
      if key == "model" {
        assert(type(value) == str, message: "Default model must be a string")
      } else if key == "deck" {
        assert(type(value) == str, message: "Default deck must be a string")
      } else if key == "tags" {
        assert(type(value) == array, message: "Default tags must be an array")
      } else if key == "format" {
        assert(
          type(value) == str and value in ("svg", "png", "plain"),
          message: "Default format must be one of: \"svg\", \"png\", \"plain\"",
        )
      } else if key == "data" {
        _validate-data(data: value)
      } else if key == "other" {
        assert(type(value) == dictionary, message: "Default other must be a dictionary")
      } else {
        panic("Unknown default key: " + key + ". Valid keys are: model, deck, tags, data, other")
      }
    }
  }

  ankify-configuration.update(config => {
    // Update configuration with provided values
    if (ankiconnect-url != none) { config.ankiconnect-url = ankiconnect-url }
    if (verbose != none) { config.verbose = verbose }
    if (defaults != none) { config.defaults = defaults }
    if (render != none) { config.render = render }
    if (cache.enabled != none) { config.cache.enabled = cache.enabled }
    if (cache.custom-file != none) { config.cache.custom-file = cache.custom-file }
    if (checks != none) { config.checks = checks }

    // Return updated configuration
    config
  })
  context {
    [#metadata(ankify-configuration.get()) <ankify-configuration>]
  }
}


/// Helper function to create a basic card with just front and back.
///
/// This is a convenience function for the most common use case.
///
/// # Arguments
///
/// - `label`: Unique identifier for the card
/// - `front`: Front side content
/// - `back`: Back side content
/// - `deck` (optional): Deck name
/// - `tags` (optional): Array of tags
///
/// # Example
///
/// ```typst
/// #basic("my-card", "Question", "Answer", deck: "Study")
/// ```
#let basic(label, front, back, deck: none, tags: none) = {
  note(
    label,
    data: (Front: front, Back: back),
    deck: deck,
    tags: tags,
  )
}

/// Helper function to create a cloze deletion card.
///
/// # Arguments
///
/// - `label`: Unique identifier for the card
/// - `text`: Text with cloze deletions (use {{c1::answer}} syntax)
/// - `deck` (optional): Deck name
/// - `tags` (optional): Array of tags
///
/// # Example
///
/// ```typst
/// #cloze("capitals", "The capital of France is {{c1::Paris}}")
/// ```
#let cloze(label, text, deck: none, tags: none) = {
  note(
    label,
    model: "Cloze",
    data: (Text: text),
    deck: deck,
    tags: tags,
  )
}
