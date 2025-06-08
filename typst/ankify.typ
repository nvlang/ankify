// See the file LICENSE for the full license governing this code.

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
///   ankiconnect_url: "http://localhost:8765",
///   verbose: true,
///   defaults: (
///     model: "Basic",
///     deck: "MyDeck",
///     tags: ("study", "typst")
///   )
/// )
/// 
/// // Create cards
/// #card(
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
/// # Arguments
///
/// - `label` (required): Unique identifier for this card
/// - `model` (optional): Anki note type (defaults to "Basic")
/// - `data` (required): Dictionary of field names to content
/// - `deck` (optional): Anki deck name (defaults to "Default")
/// - `tags` (optional): Array of tags to apply
/// - `rest` (optional): Additional metadata to pass to AnkiConnect
/// - `format` (optional): Rendering format ("svg", "png", "html", "plain")
///
/// # Examples
///
/// Basic card:
/// ```typst
/// #card(
///   "basic-example",
///   data: (
///     Front: "What is Rust?",
///     Back: "A systems programming language"
///   )
/// )
/// ```
///
/// Card with full options:
/// ```typst
/// #card(
///   "advanced-example",
///   model: "Basic (and reversed card)",
///   data: (
///     Front: "Question text",
///     Back: "Answer text",
///     Extra: "Additional information"
///   ),
///   deck: "Computer Science",
///   tags: ("programming", "rust"),
///   format: "html",
///   rest: (
///     customField: "custom value"
///   )
/// )
/// ```
#let card(
  label,
  data: dictionary,
  model: none,
  deck: none,
  tags: none,
  rest: none,
  format: none
) = {
  // Validate required arguments
  assert(type(label) == str and label != "", message: "Card label must be a non-empty string")
  assert(type(data) == dictionary and data.len() > 0, message: "Card data must be a non-empty dictionary")
  
  // Build card metadata object
  let card_data = (
    label: label,
    data: data,
    model: model,
    deck: deck,
    tags: tags,
    rest: rest,
    format: format
  )
  
  // Add optional fields if provided
  if model != none {
    assert(type(model) == str, message: "`model` must be a string")
    card_data.insert("model", model)
  }
  
  if deck != none {
    assert(type(deck) == str, message: "`deck` must be a string") 
    card_data.insert("deck", deck)
  }
  
  if tags != none {
    assert(type(tags) == array, message: "`tags` must be an array")
    card_data.insert("tags", tags)
  }
  
  if rest != none {
    assert(type(rest) == dictionary, message: "`rest` must be a dictionary")
    card_data.insert("rest", rest)
  }
  
  if format != none {
    assert(type(format) == str or type(format) == dictionary, message: "`format` must be a string or dictionary")
    if type(format) == str {
      assert(format in ("svg", "png", "html", "plain"), message: "`format` must be a dictionary or one of: svg, png, html, plain")
    }
    card_data.insert("format", format)
  }
  
  // Store as metadata for CLI extraction
  [#metadata(card_data) <anki-card>]
}

/// Configure Ankify settings for the current document.
///
/// This function stores configuration metadata that affects how the Ankify
/// CLI tool processes cards in this document. Settings specified here can
/// be overridden by CLI arguments.
///
/// # Arguments
///
/// - `ankiconnect_url` (optional): URL for AnkiConnect API
/// - `verbose` (optional): Enable verbose output
/// - `aux_file` (optional): Path to auxiliary cache file
/// - `defaults` (optional): Default values for card fields
/// - `render` (optional): Path to custom render function
///
/// # Examples
///
/// Basic configuration:
/// ```typst
/// #configure(
///   ankiconnect_url: "http://localhost:8765",
///   verbose: true
/// )
/// ```
///
/// Configuration with defaults:
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
/// Configuration with custom render function:
/// ```typst
/// #configure(
///   render: "custom-render.typ",
///   defaults: (
///     format: "html"
///   )
/// )
/// ```
#let configure(
  ankiconnect_url: none,
  verbose: none,
  aux_file: none,
  defaults: none,
  render: none
) = {
  // Build configuration object
  let config_data = (:)
  
  if ankiconnect_url != none {
    assert(type(ankiconnect_url) == str, message: "AnkiConnect URL must be a string")
    config_data.insert("ankiconnect_url", ankiconnect_url)
  }
  
  if verbose != none {
    assert(type(verbose) == bool, message: "Verbose must be a boolean")
    config_data.insert("verbose", verbose)
  }
  
  if aux_file != none {
    assert(type(aux_file) == str, message: "Aux file must be a string")
    config_data.insert("aux_file", aux_file)
  }
  
  if defaults != none {
    assert(type(defaults) == dictionary, message: "Defaults must be a dictionary")
    
    // Validate defaults structure
    for (key, value) in defaults {
      if key == "model" {
        assert(type(value) == str, message: "Default model must be a string")
      } else if key == "deck" {
        assert(type(value) == str, message: "Default deck must be a string")
      } else if key == "tags" {
        assert(type(value) == array, message: "Default tags must be an array")
      } else if key == "data" {
        assert(type(value) == dictionary, message: "Default data must be a dictionary")
      } else if key == "rest" {
        assert(type(value) == dictionary, message: "Default rest must be a dictionary")
      } else {
        panic("Unknown default key: " + key + ". Valid keys are: model, deck, tags, data, rest")
      }
    }
    
    config_data.insert("defaults", defaults)
  }
  
  if render != none {
    assert(type(render) == str, message: "Render must be a string")
    config_data.insert("render", render)
  }
  
  // Only store metadata if there's actual configuration
  if config_data.len() > 0 {
    [#metadata(config_data) <anki-config>]
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
  card(
    label,
    data: (Front: front, Back: back),
    deck: deck,
    tags: tags
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
  card(
    label,
    model: "Cloze",
    data: (Text: text),
    deck: deck,
    tags: tags
  )
}
