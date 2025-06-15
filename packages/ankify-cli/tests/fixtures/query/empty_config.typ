// This is a plain Typst file with no ankify configuration
// Used to test that the query functions return None when no ankify content is present

= Sample Document

This document contains regular Typst content but no ankify imports or configuration.

== Section 1

Some regular text content.

== Section 2

More content with some formatting:

- List item 1
- List item 2
- List item 3

#block[
  This is a block with some content.
]

$integral_0^infinity e^(-x) d x = 1$
