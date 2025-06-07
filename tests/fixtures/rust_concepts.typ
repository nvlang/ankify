#import "ankify.typ": card

// Set default options for all cards in this file
#let defaults = (
  model: "Definition",
  deck: "Computer Science",
  tags: ("programming", "rust"),
  format: "html"
)

= Rust Programming Concepts

#card(
  "ownership-definition",
  data: (
    Term: [Ownership],
    Definition: [
      Rust's system for managing memory safety without garbage collection.
      Each value has a single owner, and when the owner goes out of scope,
      the value is dropped.
    ]
  )
)

#card(
  "borrowing-definition",
  data: (
    Term: [Borrowing],
    Definition: [
      The ability to use a value without taking ownership of it.
      References allow you to refer to some value without taking ownership.
      Borrowing can be either immutable (`&T`) or mutable (`&mut T`).
    ]
  )
)

#card(
  "lifetime-definition", 
  data: (
    Term: [Lifetime],
    Definition: [
      A construct that ensures references are valid for as long as needed.
      Lifetimes prevent dangling references and ensure memory safety.
      Most lifetimes are inferred, but sometimes explicit annotation is needed.
    ]
  )
)

#card(
  "trait-definition",
  model: "Basic", // Override default model
  tags: ("programming", "rust", "traits"), // Override default tags  
  data: (
    Front: [What is a trait in Rust?],
    Back: [
      A trait defines shared behavior that types can implement.
      Similar to interfaces in other languages, traits allow for polymorphism
      and code reuse through shared method signatures.
    ]
  )
)

#card(
  "match-expression",
  data: (
    Term: [Match Expression],
    Definition: [
      A control flow construct that allows pattern matching against values.
      All possible cases must be handled (exhaustive matching), making it
      safer than switch statements in other languages.
      
      ```rust
      match value {
          Some(x) => println!("Got: {}", x),
          None => println!("Nothing"),
      }
      ```
    ]
  )
)
