// =============================================================================
//  Ankify demo — a lecture that doubles as an Anki deck.
//
//  Every definition and theorem below is written with a helper that BOTH
//  typesets the item AND registers an Anki flashcard for it. Write the notes
//  once; get a spaced-repetition deck for free.
//
//  With Anki open, sync the cards from the repository root:
//
//      ankify demo/lecture.typ
//
//  `demo/README.md` has the full setup. `lecture.typ` is also a perfectly
//  ordinary Typst document: `typst compile demo/lecture.typ`.
// =============================================================================

#import "@preview/ankify:0.1.0": note, configure, basic, cloze

// --- Ankify configuration ----------------------------------------------------
// Document-wide defaults. Every card lands in the "Real Analysis" deck, carries
// these tags, and is rendered as a crisp, theme-aware SVG — so the mathematics
// survives and the card follows Anki's light or dark mode.
#configure(
  scale: 1.6,
  defaults: (
    deck: "Real Analysis",
    tags: ("real-analysis", "lecture-5"),
    format: "svg",
  ),
)

// --- The helper pattern ------------------------------------------------------
// `note()` registers a flashcard but draws nothing on the page, so it shines
// inside a helper that ALSO typesets the item. Each `definition(..)` and
// `theorem(..)` below renders a styled block and becomes a card in one call.

#let def-color = rgb("#2f6db0")
#let thm-color = rgb("#a96a1c")

#let card-block(accent, kind, name, body) = block(
  width: 100%,
  fill: accent.lighten(93%),
  inset: (x: 1.05em, y: 0.9em),
  radius: 4pt,
  stroke: (left: 3pt + accent),
  breakable: false,
)[
  #text(fill: accent, weight: "bold")[#smallcaps(kind).] #emph(name)
  #v(0.5em, weak: true)
  #body
]

#let definition(label, term, body) = {
  note(label, data: (Front: [Define *#term*.], Back: body))
  card-block(def-color, "Definition", term, body)
}

#let theorem(label, name, statement) = {
  note(
    label,
    data: (Front: [State the *#name*.], Back: statement),
    deck: "Real Analysis::Theorems",
    tags: ("real-analysis", "lecture-5", "theorem"),
  )
  card-block(thm-color, "Theorem", name, statement)
}

// --- Page & typography -------------------------------------------------------
#set page(
  paper: "a4",
  margin: (x: 2.5cm, top: 2.6cm, bottom: 2.4cm),
  numbering: "1",
  header: context {
    if here().page() > 1 {
      set text(size: 0.82em, fill: gray.darken(15%))
      [Real Analysis · Lecture 5 #h(1fr) #emph[Convergence of Sequences]]
    }
  },
)
#set text(size: 11pt)
#set par(justify: true, leading: 0.72em, spacing: 1.2em)
#set heading(numbering: "1")
#show heading.where(level: 1): it => block(above: 1.6em, below: 0.95em)[
  #text(size: 1.2em, fill: def-color, weight: "bold")[
    #counter(heading).display() #h(0.5em) #it.body
  ]
]

// --- Title -------------------------------------------------------------------
#text(size: 1.95em, weight: "bold")[Convergence of Sequences]
#v(0.2em)
#text(size: 1.05em, fill: gray.darken(20%))[Real Analysis · Lecture 5]
#v(0.55em)
#line(length: 100%, stroke: 0.6pt + gray.lighten(15%))
#v(0.3em)

A sequence is just an unending list of numbers; whether it _settles down_ to a
value takes some care to make precise. This lecture pins the idea down and
presents two theorems guaranteeing that a limit exists.

Each coloured block below is also an Anki flashcard. The `definition` and
`theorem` helpers in this file typeset the item _and_ register a card --- so the
notes and the deck never drift apart.

= Convergence

#definition("def-convergence", "convergence")[
  A sequence $(a_n)$ _converges_ to a limit $L in RR$ if, for every
  $epsilon > 0$, there is an $N in NN$ such that $|a_n - L| < epsilon$ for all
  $n >= N$. We then write $lim_(n -> oo) a_n = L$.
]

Unpacked: every neighbourhood of $L$, however narrow, must eventually contain
the whole tail of the sequence.

#definition("def-boundedness", "boundedness")[
  A sequence $(a_n)$ is _bounded_ if there is a real number $M >= 0$ with
  $|a_n| <= M$ for every index $n in NN$.
]

#definition("def-monotonicity", "monotonicity")[
  A sequence $(a_n)$ is _monotone_ if it is non-decreasing --- $a_n <= a_(n+1)$
  for all $n$ --- or non-increasing --- $a_n >= a_(n+1)$ for all $n$.
]

= Existence theorems

The definition of convergence presupposes a limit $L$ that you already know.
The next two results instead manufacture a limit out of structure alone.

#theorem("thm-monotone-convergence", "Monotone Convergence Theorem")[
  Every bounded, monotone sequence converges. If $(a_n)$ is non-decreasing and
  bounded above, then $lim_(n -> oo) a_n = sup_n a_n$.
]

#theorem("thm-bolzano-weierstrass", "Bolzano–Weierstrass Theorem")[
  Every bounded sequence of real numbers has a convergent subsequence.
]

#block(
  width: 100%,
  fill: luma(96%),
  inset: (x: 1.05em, y: 0.9em),
  radius: 4pt,
  stroke: (left: 3pt + luma(70%)),
  breakable: false,
)[
  #text(fill: luma(35%), weight: "bold")[#smallcaps[Example].]
  #v(0.5em, weak: true)
  The sequence $a_n = 1 - 1 / n$ is increasing and bounded above by $1$, so the
  Monotone Convergence Theorem applies: it converges, and
  $ lim_(n -> oo) (1 - 1 / n) = sup_n (1 - 1 / n) = 1. $
  This block is typeset by hand --- with no `note()` call --- a reminder that
  you decide what becomes a card.
]

= Quick review

`note()` has two shorthands. `basic(label, front, back)` builds a simple
front-and-back card, and `cloze(label, text)` builds a fill-in-the-blank card
from `{{c1::..}}` markers --- always as plain text, so Anki receives the markers
verbatim and can turn them into blanks.

#basic(
  "review-limit-uniqueness",
  "Can a sequence converge to two different limits?",
  "No. The limit of a convergent sequence is unique.",
)

#cloze(
  "review-convergent-bounded",
  "Every {{c1::convergent}} sequence is {{c2::bounded}}.",
)

Two prompts to test yourself --- each is also one of the cards registered just
above:

+ Can a sequence converge to two different limits?
+ Is every convergent sequence bounded?

#v(1fr)
#line(length: 100%, stroke: 0.6pt + gray.lighten(15%))
#v(0.35em)
#text(size: 0.86em, fill: gray.darken(12%))[
  Typeset with Typst, synced to Anki with
  #link("https://github.com/nvlang/ankify")[`ankify`]. Edit a card and re-run
  `ankify demo/lecture.typ` --- only changed cards are re-sent.
]
