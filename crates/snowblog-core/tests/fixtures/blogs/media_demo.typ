#metadata(
  (
    title: "A Media Demonstration",
    description: "Images inside align and pad wrappers.",
    date: "2024-03-10",
    tags: ("demo", "media"),
    hidden: false,
  ),
)<frontmatter>

#set page(height: auto, margin: 0.7em)

= Pictures

#align(
  center,
)[#image("./assets/photo.jpg", width: 50%) (A caption under a centered photograph)]

Some prose between the pictures keeps the layout honest.

#align(center)[
  #image("./assets/blue_square.png", width: 40%, alt: "A small blue square used as a stand-in illustration")
  (Another caption)
]

#pad(x: 2em)[#image("./assets/dot.png", width: 10%)]
