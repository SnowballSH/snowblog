#metadata(
  (
    title: "A Graphics Demonstration",
    description: "A cetz canvas that must survive HTML export.",
    date: "2024-04-05",
    tags: ("demo",),
    // hidden: true,
  ),
)<frontmatter>

#set page(height: auto, margin: 0.7em)

#import "@preview/cetz:0.5.2"

= A triangle

#context cetz.canvas({
  import cetz.draw: *
  line((0, 0), (2, 0))
  line((2, 0), (1, 1.5))
  line((1, 1.5), (0, 0))
  circle((1, 0.5), radius: 0.1)
})

That triangle is drawn with cetz and, after import adaptation, rendered as an
inline SVG frame.
