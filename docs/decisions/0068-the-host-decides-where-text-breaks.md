# Decision 0068: The host decides where text breaks

**Status:** Accepted

**Date:** 2026-08-12

## Context

A text node's value is one line. The model refuses control characters, so a
newline cannot be written into a document, and until now the host drew each
value as one run and let the client rectangle cut off whatever ran past the
edge.

That was not a rough edge, it was a hole. A sentence of ordinary length in a
window of ordinary width disappeared mid-word. It was found the way such things
usually are: a person ran the field diagnostic, took a screenshot, and the last
word of the explanation was `applicatio`. Every document was affected, because
every document has prose in it.

The question is not whether to wrap. It is who decides where.

An application could decide, by carrying the break points in the document. That
is what a fixed-width layout does, and it is wrong here for the same reason
Anodrel does not let an application position its own window: the application
does not know how wide the window is, must not have to know, and would be wrong
the moment the person resized it. It would also make a document dependent on the
font the host happened to have.

The host could decide, at paint time only, and never tell layout. That keeps the
document clean but leaves the measured height wrong, so a wrapped paragraph
would overlap whatever came next.

## Decision

**Wrapping is presentation, and the host owns it end to end.**

A text value stays one line as content. The host breaks it into as many visual
lines as the column it was given requires, and layout measures it as that block
of lines, so a paragraph that grew pushes its siblings down instead of drawing
over them.

The breaking lives in `anodrel-ui`, not in a host adapter. It is portable code
tested against a predictable measurer rather than a real font, and — more
importantly — it is one function that both layout and painting call, so the
height that was reserved and the lines that are drawn come from the same place.

Consequences that follow from making this presentation rather than content:

- **No protocol version, no grant, no document change.** A document written
  before this decision renders correctly after it, and renders differently at
  two window widths without either being wrong.
- **An application never learns the break points.** They appear in no document,
  no observable layout, and no event. This is the same rule as
  Decision 0067's: the application supplies the words, and what happens to them
  on screen is not its business.
- **Layout reports a run's width as its widest line**, not the column it wrapped
  against, so a short label does not inflate a stack's cross axis. Greedy
  breaking makes that safe — a word rejected at the wider width is still
  rejected at the narrower one — and the invariant is held by a test rather
  than by a comment.
- **A word wider than the column is broken between characters.** Leaving it
  whole would reintroduce exactly the clipping this removes.
- **`MAX_TEXT_LINES` bounds the result.** A long value in a narrow column could
  otherwise produce a line per character and a surface tall enough to stall a
  paint. Text past the limit stays on the final line and is clipped, which is
  what every value did before — bounded, and never silently truncated.

## Alternatives considered

**Break points in the document.** Rejected: it makes the application responsible
for a width it cannot know, and freezes a layout that should reflow.

**A `wrap: bool` on the text node.** Rejected as a false choice. It asks the
application whether it would like its text cut off, and nothing sensible answers
yes. Wrapping is what a text run does; a node that deliberately did not wrap
would be a different node with a different name.

**A multi-line value with embedded newlines.** Rejected for now. It is a content
change, not a presentation one — it would need the model's control-character
rule reopened, and it does not solve this problem, since a long line still has
to wrap.

**Wrapping only at paint time.** Rejected: the measured height would stay
single-line and every wrapped paragraph would overlap its neighbour.

## Consequences

`docs/UI.md` gains a wrapping section. Multi-line values, justification,
hyphenation, locale-aware line breaking, bidirectional text, and horizontal
overflow remain unaddressed and each need their own decision. Line breaking here
is space-based, which is correct for the Latin scripts the platform has been
tested with and is not a claim about scripts that do not break at spaces.
