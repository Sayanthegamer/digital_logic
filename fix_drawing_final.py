import re

with open("src/editor/drawing.rs", "r") as f:
    code = f.read()

# Update seg states size
code = code.replace("let mut seg_states = [false; 7];", "let mut seg_states = [false; 8];")
code = code.replace("if comp.comp_type == ComponentType::SevenSegment && i < 7 {", "if comp.comp_type == ComponentType::SevenSegment && i < 8 {")

# Add the drawing code
code = code.replace(
"""                // Segment G (middle)
                draw_line(
                    cx - w,
                    cy,
                    cx + w,
                    cy,
                    thick,
                    seg_color(seg_states[6]),
                );""",
"""                // Segment G (middle)
                draw_line(
                    cx - w,
                    cy,
                    cx + w,
                    cy,
                    thick,
                    seg_color(seg_states[6]),
                );

                // Segment Minus (top left, extra neg sign)
                draw_line(
                    cx - w - 20.0 * self.canvas.zoom,
                    cy,
                    cx - w - 10.0 * self.canvas.zoom,
                    cy,
                    thick,
                    seg_color(seg_states[7]),
                );"""
)

with open("src/editor/drawing.rs", "w") as f:
    f.write(code)
