#import "packages/ankify-typst/lib.typ": note, configure

#configure(verbose: true)

#note(
  label: "pythagoras-theorem",
  deck: "Ankify-Test",
  model: "Basic",
  format: "png",
  data: (
    Front: "What is the Pythagorean theorem?",
    Back: [test $lim_(n -> infinity) n / 2$ ... #rect(height: 100pt, fill: blue)],
  ),
)

#note(
  label: "quadratic-formula",
  deck: "Ankify-Test",
  model: "Basic",
  format: "svg",
  data: (
    Front: [What is the quadratic formula?],
    Back: [For $a x^2 + b x + c = 0$: $ x = 1 / 2 $],
  ),
)

