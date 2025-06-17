//! This module is the heart of Ankify. It'll provide a `sync` function that is
//! the sole (or at least main) public API of the Ankify library and binary
//! crates. This `sync` function should operate in the following steps:
//!
//! 1.  *Check AnkiConnect.*
//!     Check if the AnkiConnect server is running. If not, return an error.
//!     Otherwise, continue.
//!
//! 2.  *Generate temp file and read cache.*
//!     In parallel:
//!
//!     -   Call the `generate` module to generate the temporary Typst files
//!         that will be used to create the Anki notes.
//!     -   Call the `cache` module to load the cache of existing notes.
//!
//! 3.  *Calling Typst.*
//!     In parallel:
//!
//!     -   Call the `query` module to query the Typst source file for metadata
//!         about the user's Ankify settings, as well as about the notes to be
//!         created.
//!     -   If the cache has any fields in PNG/SVG format (and remember, PNG is
//!         the default), call the `compile` module on the generated Typst file
//!         with the corresponding output format flags. If both PNG and SVG
//!         files are being output, be sure to run the two compilations in
//!         parallel.
//!
//! 4.  *Pick output files.*
//!     Use the output from the `query` module to determine which output files
//!     are relevant.
//!
//! 5.  *Hashing.*
//!     In parallel: Hash each of the relevant output files.
//!
//! 6.  *Decision-making.*
//!     Create a `RequestList` that contains the requests (or lists of requests)
//!     to be sent to be sent to AnkiConnect. To do so, compare the list of
//!     notes provided by the `query` module with the notes in the cache to
//!     determine which notes are new and need to be added to Anki for the first
//!     time, and which notes are already in Anki and simply need to be updated.
//!     Then, taking advantage of the `ankiconnect` module, do the following:
//!
//!     -   For new notes:
//!
//!         1.  Check if the notes' decks are the same as the decks of any
//!             other notes in the cache. If so, we can assume that the
//!             decks already exist in Anki; otherwise, we need to send a
//!             `deckNamesAndIds` request to AnkiConnect to check if the
//!             decks exist, and, if they (or at least some of them) don't,
//!             we need to send a `createDeck` request (or multiple
//!             `createDeck` requests, grouped into a single `RequestList`
//!             with `multi` set to `true`) _before_ adding the notes.
//!         2.  Create an `addNotes` request for the new notes.
//!
//!     -   For existing notes:
//!
//!         -   Compare the new hashes with the hashes from the cache to
//!             determine which output files ones need to be updated in the
//!             Anki database. Note that some fields may not have had an
//!             output file associated with them in the cache, which would
//!             mean that they were either omitted before, or merely
//!             contained plain text. In either case, if the new note has an
//!             output file associated with the field, we should update the
//!             field in the Anki database accordingly. Conversely, if the
//!             cache has an output file associated with the field, but the
//!             new note does not, we should remove the field from the Anki
//!             database; if the new note has plain text in the field, then
//!             we should update the field in the Anki database with the
//!             plain text.
//!         -   Compare the tags of the new note with the tags in the cache
//!             to determine if the tags need to be updated in the Anki
//!             database.
//!
//!         The `updateNote` requests should be grouped into a single
//!         `RequestList` with the `multi` field set to `true`, so that
//!         the requests are sent to AnkiConnect simultaneously.
//!
//!     Note that, if a note's field has an output file associated with it, then
//!     the field should be set to `"<img class=\"ankify\"
//!     src=\"〈output-file〉\"/>"` in the note's `fields` field, and the output
//!     file should be included _in the same request_ (be it an `addNotes` or an
//!     `updateNote` request) in the `picture` array, with `url` set to the path
//!     to the output file, `filename` set to `"〈output-file〉"`, and `fields`
//!     set to `["〈field〉"]`.
//!
//! 7.  *Execution.* Process the `RequestList` and send the corresponding
//!     requests AnkiConnect. Make use of the `ankiconnect` module to understand
//!     the responses from AnkiConnect. While doing all this, be sure to handle
//!     any errors that may occur, and keep the cache up to date with the
//!     changes made to the Anki database (i.e., when `addNotes` or `updateNote`
//!     requests succeed).
//!
//!     If this is running in a CLI context (i.e., if it's called from the
//!     binary crate), then we furthermore want to provide some pretty output to
//!     the user while all this is happening, including progress bars, error
//!     messages, and so on. If the `verbose` setting is enabled in the Ankify
//!     configuration (as returned by the `query` module), or if the user set
//!     the `--verbose` flag in the CLI, we should provide more detailed output
//!     than we would otherwise.
//!
//! 8.  *Cleanup.*
//!     Delete the temporary file that were generated in step 2, and the output
//!     files that were produced in step 3.

pub struct RequestList {
    /// Indicates whether the requests should be sent to AnkiConnect
    /// inside of a `"multi"` request or not.
    pub multi: bool,

    /// List of requests or lists of requests to be sent to AnkiConnect.
    pub requests: Vec<RequestOrRequestList>,
}

pub enum RequestOrRequestList {
    /// A single request to be sent to AnkiConnect.
    Single(serde_json::Value),

    /// A list of requests to be sent to AnkiConnect. Note that the `sequential`
    /// field must be respected when processing this list.
    List(RequestList),
}
