#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "Plain-Test",
    model: "Basic",
    format: "plain",
    tags: ("plain", "text"),
  ),
)

#note(
  "basic-fact",
  data: (
    Front: "What is the capital of Japan?",
    Back: "Tokyo",
  ),
)

#note(
  "simple-math",
  data: (
    Front: "What is 15 x 8?",
    Back: "120",
    Extra: "This is basic multiplication",
  ),
)

#note(
  "definition",
  data: (
    Front: "Define photosynthesis",
    Back: "The process by which plants convert light energy into chemical energy",
    Source: "Biology textbook",
  ),
)

= Plain Text Only Document

This document contains only plain text format notes for testing.
No compilation should be needed since all content is text-based.

These notes are suitable for simple Q&A flashcards where no special
formatting, mathematics, or graphics are required.

The content should be passed directly to Anki without any image generation.
