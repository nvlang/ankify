#import "source.typ"
#import "ankify.typ": __ankify-configuration, __ankify-notes
#hide([#source])
#set page(height: auto)

#context {
  (__ankify-configuration.final().setup)()
  for note in __ankify-notes.final() {
    for (field, value) in note.data {
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
