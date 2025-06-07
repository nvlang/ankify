#import "ankify.typ": card

// Configuration for this document
#let config = (
  ankiconnect_url: "http://localhost:8765",
  verbose: true,
  aux_file: "physics.aux.json",
  render: "custom_render.typ"
)

= Physics Fundamentals

This document contains physics flashcards with various formats.

#card(
  "newton-first-law",
  model: "Basic",
  deck: "Physics",
  tags: ("mechanics", "newton-laws"),
  format: "svg", // Will render equations as SVG
  data: (
    Front: [State Newton's First Law of Motion],
    Back: [
      An object at rest stays at rest, and an object in motion stays in motion
      at constant velocity, unless acted upon by a net external force.
      
      Mathematically: $sum F = 0 => v = "constant"$
    ]
  )
)

#card(
  "newton-second-law",
  model: "Basic", 
  deck: "Physics",
  tags: ("mechanics", "newton-laws", "force"),
  format: "svg",
  data: (
    Front: [What is Newton's Second Law?],
    Back: [
      The acceleration of an object is directly proportional to the net force
      acting on it and inversely proportional to its mass.
      
      $ F = m a $
      
      Where $F$ is force, $m$ is mass, and $a$ is acceleration.
    ]
  )
)

#card(
  "kinetic-energy-formula",
  model: "Cloze",
  deck: "Physics", 
  tags: ("energy", "mechanics"),
  format: "png", // Will render as PNG image
  data: (
    Text: [
      The kinetic energy of an object with mass $m$ and velocity $v$ is:
      {{c1::$K E = frac(1, 2) m v^2$}}
    ]
  )
)

#card(
  "wave-equation",
  model: "Basic",
  deck: "Physics",
  tags: ("waves", "equations"),
  format: "html", // Will render as HTML with MathJax
  rest: (
    difficulty: "medium",
    source: "Griffiths Physics"
  ),
  data: (
    Front: [What is the general wave equation?],
    Back: [
      The one-dimensional wave equation is:
      
      $ frac(partial^2 y, partial t^2) = v^2 frac(partial^2 y, partial x^2) $
      
      Where $y(x,t)$ is the wave function, $v$ is the wave speed,
      $x$ is position, and $t$ is time.
    ]
  )
)

#card(
  "simple-definition",
  model: "Basic",
  deck: "Physics",
  tags: ("definitions",),
  format: "plain", // No rendering, just plain text
  data: (
    Front: "What is velocity?",
    Back: "Velocity is the rate of change of displacement with respect to time. It is a vector quantity with both magnitude and direction."
  )
)
