---
name: tui-screen
description: Add or change a screen in the interactive interface - state, key bindings, layout, scrolling, mouse handling and TestBackend rendering tests. Use for any edit to src/tui.rs, or when the UI needs a new menu entry, checkbox row, list, log view or key binding.
---

# Working on the interactive interface

Everything lives in `src/tui.rs`: one `App` struct holds the state, `Screen` selects which
`draw_*` and `on_*_key` pair is active, and the tests at the bottom render real frames into a
`TestBackend`.

## Adding a screen

1. Add a variant to `Screen`.
2. Add its fields to `App` and initialise them in `App::new`.
3. Route it: a match arm in `on_key` and one in `draw`.
4. Reset what needs resetting in `open()` (focus, notices, scroll offsets) and make sure
   `back_home()` clears anything that should not survive leaving the screen.
5. If it is reachable from the menu, add it to `HOME_ITEMS` *and* to the `KeyCode::Enter`
   match in `on_home_key` - the two are positional and must stay in step.

## Key handling rules

- Ctrl+C quits from anywhere; it is handled in `on_key` before the screen dispatch.
- Guarded arms come first. `Enter if focus == LAST => start()`, then a plain `Enter` for
  everything else. Reordering these silently changes what Enter does.
- A text-input row is matched before `Char(' ')` toggles, otherwise the space bar is eaten by
  the checkbox handling instead of typing a space.
- While a job is running the screen must not respond to navigation or "go back":
  `on_running_key` only acts once `done` is set, because interrupting mid-install is worse
  than an unresponsive key.
- Non-selectable rows (category headers) are skipped in `starter_move`, so focus never lands
  on one.

## Scrolling

Two conventions, deliberately different:

- **Lists** (`starter_scroll`) are top-relative. `starter_reveal()` runs after every move and
  nudges the offset so the focused row is inside the window. `view_height` is a
  `Cell<usize>` filled during `draw` - key handling needs to know how much fits.
- **The log** (`log_scroll`) is bottom-relative: `0` follows the newest line. When new output
  arrives while the user is scrolled back, `drain` increments `log_scroll` by the number of
  new lines so the line being read stays in place. Dropping that increment makes the view
  drift under the reader.
- **The wheel means opposite things in the two views**: back through a log, down a list. That
  asymmetry lives in `on_mouse` and is intentional.

## Never block the loop

Anything slow (winget, filesystem sweeps, network) goes through `run_job(title, closure)`.
The closure gets a `Console` and runs on its own thread; `tick` drains the channel into the
log and picks up the final result. Keys stay responsive and the spinner advances while it
runs.

## Rendering

- Use `panel(frame, area, title, lines)` - it applies the border and horizontal padding that
  every other screen uses.
- `row_line` (selectable), `checkbox_line` (selectable + state), `action_line` (the commit
  row), `warning_line` (a notice), `footer_line` (key hints) cover the existing vocabulary;
  extend it only if a screen genuinely needs a new one.
- Keep text ASCII - legacy consoles run codepage 437.

## Tests

```rust
let mut app = app();          // no bundles, no ambient state
app.screen = Screen::Starter;
let screen = render(&app);    // 90x24 TestBackend frame as text
assert!(screen.contains("Browsers"));
press(&mut app, KeyCode::End);
```

Add tests for: the screen renders its rows, focus movement skips what should not be
focusable, the action refuses to start when nothing is selected, the hints line mentions the
keys, and any scroll behaviour you added. `wheel(&mut app, MouseEventKind::ScrollUp)` for
mouse paths.

```powershell
cargo test tui
```

Tests must not start a real job unless it is a dry run - `starter_screen_runs_a_dry_run_to_the_end`
is the pattern for exercising that path safely.
