#import "ankify.typ": card

// Test file with configuration that should be merged with CLI args
#let config = (
  ankiconnect_url: "http://localhost:9999", // Different from default
  verbose: true,
  defaults: (
    model: "TestModel",
    deck: "TestDeck", 
    tags: ("test", "integration"),
    format: "svg"
  )
)

= Test Cards for Integration Testing

#card(
  "test-card-1",
  data: (
    Front: [Test question 1],
    Back: [Test answer 1]
  )
)

#card(
  "test-card-2", 
  model: "Override", // Should override default
  deck: "SpecialDeck", // Should override default
  tags: ("special", "override"), // Should override default
  data: (
    Question: [What is 2 + 2?],
    Answer: [4]
  )
)

#card(
  "test-card-with-rest",
  rest: (
    priority: "high",
    difficulty: 5,
    custom_field: "custom_value"
  ),
  data: (
    Front: [Question with extra metadata],
    Back: [Answer with metadata]
  )
)
