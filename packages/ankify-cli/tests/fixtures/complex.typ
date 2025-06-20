#import "../../../ankify-typst/lib.typ": note, configure

#configure()

#note(
    "note-plain-plain",
    data: (
        Front: "What is the capital of France?",
        Back: "Paris"
    ),
    format: "plain"
)

#note(
    "note-plain-svg",
    data: (
        Front: "Pythagoras' theorem",
        Back: (
            format: "svg",
            value: [$a^2 + b^2 = c^2$]
        )
    ),
    format: "plain"
)

#note(
    "note-svg-plain",
    data: (
        Front: (
            format: "svg",
            value: "A"
        ),
        Back: "B"
    ),
    format: "plain"
)

#note(
    "note-png-png",
    data: (
        Front: "What is the capital of England?",
        Back: "London"
    )
)

#note(
    "note-plain-png",
    data: (
        Front: "Cauchy's integral theorem",
        Back: (
            format: "svg",
            value: [
                $integral.cont f(z) dif z = 0$ for any closed curve $C$ in a simply connected domain where $f$ is analytic.
            ]
        )
    )
)

#note(
    "note-svg-png",
    data: (
        Front: (
            format: "svg",
            value:[$Z^ast$ theorem]
        ),
        Back: (
            format: "png",
            value: [
                Let $G$ be a finite group, with $O(G)$ being its maximal normal subgroup of odd order.
                If $T$ is a Sylow 2-subgroup of $G$ containing an involution not conjugate in $G$
                to any other element of T, then the involution lies in $Z^ast(G)$,
                which is the inverse image in G of the center of $G/O(G)$.
            ]
        )
    )
)

complex.typ
