#import ".ankify/ankify-typst/lib.typ": note, configure

#configure(
    setup: () => {
        set page(margin: 1cm, width: 16cm)
    }
)

#note(
    "pythagorean-theorem",
    data: (
        Front: "What is the Pythagorean theorem?",
        Back: (
            format: "svg",
            value: [$a^2 + b^2 = c^2$]
        )
    ),
    format: "plain"
)

#note(
    "einstein-equation",
    data: (
        Front: "Einstein's mass-energy equation",
        Back: (
            format: "png",
            value: [$E = m c^2$]
        )
    ),
    format: "plain"
)

#note(
    "plain-text-note",
    data: (
        Front: "What is the capital of France?",
        Back: "Paris"
    ),
    format: "plain"
)
