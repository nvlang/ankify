

#let _validate-data(data: dictionary) = {
  assert(type(data) == dictionary, message: "Note data must be a dictionary")
  assert(data.len() > 0, message: "Note data dictionary must not be empty")
  for (key, value) in data {
    assert(type(key) == str, message: "Keys of note data dictionary must be strings (corresponding to field names)")
    assert(
      type(value) in (content, str, dictionary),
      message: "Values of note data dictionary must be content, strings, or dictionaries",
    )
    if type(value) == dictionary {
      // Ensure dictionary values are not empty
      assert(value.len() > 0, message: "Values of note data dictionary must not be empty dictionaries")
      for (subkey, subvalue) in value {
        assert(
          type(subkey) in ("value", "format"),
          message: "Keys of dictionary values of note data dictionary must be \"value\" or \"format\"",
        )
        if (subkey == "value") {
          assert(
            type(subvalue) in (content, str),
            message: "Value of \"value\" in dictionary value of note data dictionary must be content or string",
          )
        } else if (subkey == "format") {
          assert(
            type(subvalue) in ("png", "svg", "plain"),
            message: "Value of \"format\" in dictionary value of note data dictionary must be \"png\", \"svg\", or \"plain\"",
          )
        }
      }
    }
  }
}
