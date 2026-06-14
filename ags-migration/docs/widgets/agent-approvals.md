# Agent Approvals

## Responsibilities

The Agent Approval widget is a full-screen overlay for answering pending agent
approval requests.

It presents every pending approval from active agent sessions. A session
contributes approvals only when it requires attention. Multiple pending
requests from the same session are shown as separate approval cards when the
session exposes request ids and per-request prompt/detail/option lists. Older
single-request sessions fall back to the session-level pending prompt, detail,
and options.

Each approval card shows:

- Project/workspace identity, represented by a project icon when available.
- The session working directory, shortened by replacing the home directory with
  `~`; an empty path reads as `unknown cwd`.
- Agent display name, falling back to `agent`.
- Model name when present.
- Approval prompt text.
- Optional detail content.
- One or more answer options.

If an approval has no explicit options, the default answers are `Allow` and
`Deny`. Option descriptions are shown under their option label when present.
Choosing an option sends that exact answer back to the session. After an answer,
the answered request is removed from the current overlay contents. The overlay
hides when no requests remain.

When there are no pending approvals, the overlay can still open and shows a
single empty state card reading `No pending approvals`.

The overlay supports manually showing, hiding, toggling, and opening directly to
a target session. When opened for a target session, the carousel scrolls to the
first pending request for that session.

## Input Behavior

The overlay takes exclusive keyboard focus while visible.

Keyboard behavior:

- `Escape` hides the overlay.
- `Enter`, keypad `Enter`, or `Space` submits the currently selected option for
  the current approval.
- `Left` or `h` moves to the previous approval card.
- `Right` or `l` moves to the next approval card.
- `Up` or `k` moves option selection upward.
- `Down` or `j` moves option selection downward.
- Number keys `1` through `9` immediately submit the corresponding option.

Pointer behavior:

- Approval cards are arranged in a horizontally draggable carousel.
- Scroll wheel movement can move between cards.
- Clicking an option selects and submits it.
- Focusing an option updates the selected option highlight.

## Detail Rendering

Detail content is optional. Empty detail text omits the detail area.

Diff details use syntax highlighting with colors derived from the active GTK
theme. Added lines, removed lines, hunk headers, file headers, selections, and
general source tokens receive distinct theme-derived colors.

Command details use shell syntax highlighting.

Structured text details such as diff, command, and JSON use monospace text and
do not wrap. Other text details use normal text wrapping.

Long content appears inside a scrollable region. Source-style detail views size
to their line count up to a bounded height, while the card content region has a
maximum visible height before scrolling.

## Visual Design

The overlay covers the whole monitor on the overlay layer with a translucent
black scrim.

Approval cards sit centered within the overlay carousel. Cards use the OSD
surface color, elevation, 12px radius, 20px padding, and horizontal margins.

The card header is compact and horizontal: project icon on the left, project
path and agent name in the middle, and model name on the right. The project path
is bold and slightly larger than body text. Agent and model labels are muted.

Prompt text is slightly larger than normal body text and wraps.

Detail surfaces are lightly tinted, rounded, and inset with padding. Monospace
detail text uses the system monospace font.

Options are full-width rows with transparent default backgrounds, 8px radius,
and a leading numeric key hint. A selected option receives an accent-tinted
background, with stronger tint on hover and active states. Option descriptions
are smaller and muted.

Carousel indicator dots are shown below the carousel.

## Auto-Open Behavior

The overlay can open automatically when the selected workspace has a linked
agent session with a pending approval. The auto-open target is the matching
session, so the carousel opens directly to that session's approval.

Auto-open is de-duplicated by session id and pending prompt. A prompt that has
already opened for a session does not open the overlay again until the key
changes.
