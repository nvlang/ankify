#import "../../../../ankify-typst/lib.typ": note, configure

// Advanced configuration with all options
#configure(
  ankiconnect-url: "http://advanced:9000",
  verbose: true,
  render: "advanced-render",
  cache: (
    enabled: true,
    custom-file: "advanced.json",
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
  defaults: (
    model: "Advanced",
    deck: "Advanced-Deck",
    tags: ("advanced", "test"),
    format: "svg",
  ),
)

// Advanced note with multiple features
#note(
  label: "advanced-note",
  deck: "Advanced-Test",
  model: "Cloze",
  format: "png",
  tags: ("math", "advanced", "geometry"),
  data: (
    Text: [In a right triangle, if the legs have lengths $a$ and $b$, then the hypotenuse has length {{c1::$sqrt(a^2 + b^2)$}}],
    Extra: [This is the Pythagorean theorem],
  ),
  other: (
    source: "Geometry textbook",
    difficulty: "intermediate",
  ),
)

// Another advanced note
#note(
  label: "complex-math",
  deck: "Advanced-Test",
  model: "Basic",
  format: "svg",
  tags: ("calculus", "limits"),
  data: (
    Front: [What is $lim_(x -> 0) (sin x) / x$?],
    Back: [The limit is $1$. This is a fundamental limit in calculus.],
  ),
)
