#import "lib.typ": note, configure

a

#configure()

b

#note(
  "pythagoras-theorem",
  deck: "Ankify-Test",
  model: "Basic",
  format: "png",
  data: (
    Back: [test $lim_(n -> infinity) n / 2$ ... #rect(height: 100pt, fill: blue)],
    Front: "What is the Pythagorean theorem?",
  ),
)

c1

#configure(
  defaults: (
    render: (note: none, field: none, field-content: none) => {
      [#note.at("data").at(field) heyyyyyyyy!!!]
    },
  ),
)

c2

#note(
  "quadratic-formula",
  deck: "Ankify-Test",
  model: "Basic",
  format: "svg",
  data: (Front: [What is the quadratic formula?], Back: [For $a x^2 + b x + c = 0$: $ x = 1 / 2 $]),
)

d

// fdsfds @pythagoras-theorem and @quadratic-formula
