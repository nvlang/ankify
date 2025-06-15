// Custom render function for testing
#let render(cardObj, field) = {
  // Get the field content
  let content = cardObj.data.at(field, default: "")
  
  // Add some styling based on the format
  if cardObj.format == "svg" {
    // For SVG format, add nice styling
    set text(size: 12pt)
    set par(justify: true)
    
    block(
      fill: rgb("#f8f9fa"),
      inset: 8pt,
      radius: 4pt,
      stroke: rgb("#dee2e6"),
      content
    )
  } else if cardObj.format == "html" {
    // For HTML format, simpler styling  
    set text(size: 11pt)
    content
  } else if cardObj.format == "png" {
    // For PNG format, larger text for better readability
    set text(size: 14pt, weight: "bold")
    content
  } else {
    // Plain format - return as-is
    content
  }
}
