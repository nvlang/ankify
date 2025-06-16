//! This module is responsible for generating temporary Typst files that will
//! then be compiled by the `compile` module.
//!
//! The idea is essentially this:
//!
//! ```typst
//! #import source.typ
//! #import source.typ: ankify-notes, ankify-configuration
//! #hide([#source])
//! #set page(height: auto)
//!
//! #context (
//!   #let (
//!     ankify-setup: setup,
//!     ankify-render: render
//!   ) = ankify-configuration.final()
//!
//!   #setup()
//!
//!   for c in ankify-notes.final() {
//!     for (field, value) in c.data {
//!       render(c, field)
//!       pagebreak(weak: false)
//!     }
//!   }
//! )
//! ```
//!
