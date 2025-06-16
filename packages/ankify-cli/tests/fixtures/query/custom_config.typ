#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  ankiconnect-url: "http://custom:9999",
  verbose: true,
  cache: (
    enabled: false,
    custom-file: "custom.json",
  ),
  checks: (
    typst: false,
    ankiconnect: (
      model: false,
      deck: false,
      tags: false,
    ),
  ),
)

#note(
  "custom-note",
  deck: "Custom-Deck",
  model: "Custom-Model",
  format: "plain",
  tags: ("custom", "test"),
  data: (
    Front: "Custom question",
    Back: "Custom answer",
  ),
)
