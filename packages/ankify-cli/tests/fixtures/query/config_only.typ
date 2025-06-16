#import "../../../../ankify-typst/lib.typ": note, configure

// Configuration only file - has ankify configuration but no notes
#configure(
  ankiconnect-url: "http://localhost:8765",
  verbose: true,
  cache: (
    enabled: true,
    custom-file: none,
  ),
  checks: (
    typst: true,
    ankiconnect: (
      model: true,
      deck: true,
      tags: true,
    ),
  ),
  defaults: (
    model: "Basic",
    deck: "Default-Deck",
    format: "png",
    tags: ("default",),
  ),
)

// This file contains only configuration, no notes
// Used to test that query_ankify_configuration returns the config
// but query_ankify_notes returns an empty array

= Configuration Only Document

This document demonstrates ankify configuration without any notes.
It's used to test the behavior when a file has configuration but no note content.

== Sample Content

Here's some regular Typst content:

$integral_(-infinity)^infinity e^(-x^2) d x = sqrt(pi)$

#block[
  This document has ankify configuration but no notes.
]
