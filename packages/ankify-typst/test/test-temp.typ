#import "../source.typ" as src
#import "../ankify.typ": __ankify-configuration, __ankify-notes
#set page(height: auto)
#hide([#src])

#context {
  (__ankify-configuration.final().setup)()
  for note in __ankify-notes.final() {
    let sorted-data = note.data.pairs().sorted()
    for (field, value) in sorted-data {
      let field-content = none
      if (type(value) == dictionary and "value" in value) {
        field-content = value.value
      } else if (type(value) == content or type(value) == str) {
        field-content = value
      } else {
        panic("Invalid type for note data field", field)
      }
      (note.render)(note: note, field: field, field-content: field-content)
      pagebreak(weak: false)
    }
  }
}
