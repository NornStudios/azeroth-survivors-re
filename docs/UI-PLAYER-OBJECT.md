# Running survivors on the client's own interface

What this branch is for. `main` draws the mode's own HUD; the game this is a build of draws the
real 1.12 interface — unit frames, action bar, minimap, chat — and that is most of why it reads as
WoW. The engine under us can do it: `benilla-ui` parses FrameXML, and `benilla-app` carries the
`ui_*` modules that feed it. The mode just never switched it on.

This branch switched it on and found out what that costs. It is **not** mergeable: the interface
loads and binds, and then throws Lua errors, because FrameXML is written for a logged-in character
and the mode has no player object worth the name.

## What was in the way, in the order it appeared

Each of these was one line, and each was invisible until the one before it was fixed.

1. **The interface never loaded.** `SurvivorsPlugin` inserts `run_mode::CaptureMode` to keep the
   engine's player and camera systems off the hero it drives itself (`survivors/mod.rs`), and
   `ui_script::lifecycle::ui_wanted` read that flag as "screenshot harness, skip FrameXML". The
   gate now asks whether the UI is wanted, not whether the flag is set.
2. **The frames bound to nothing.** `"player"` resolves through `net::SelfGuid`, which only a login
   sets. The hero sets it now.
3. **The engine's own feeds skipped the hero.** They query the `net::SelfPlayer` marker, which only
   a login inserts. The hero carries it now.
4. **The frames read the wrong numbers.** `units::spawn_unit` seeds every unit 100/100 at the spec's
   level; the run's real health and level live in `Run`. `hero::feed_player_frame` pushes health,
   max health, level and the XP pair into the unit's fields when they change.

## What the interface actually asks about the player

Finite, and shorter than it feels. Every `player_*` accessor the UI modules read:

| what | field | who reads it |
| --- | --- | --- |
| `PLAYER_XP` | 716 | `UnitXP("player")`, the XP bar |
| `PLAYER_NEXT_LEVEL_XP` | 717 | `UnitXPMax("player")`, the XP bar |
| `PLAYER_FLAGS` | 190 | four separate reads: ghost, group leader, resting, PvP |
| `PLAYER_REST_STATE_EXPERIENCE` | 1175 | `ExhaustionTick_Update` |
| rest state | `PLAYER_BYTES_2` high byte | `ExhaustionTick_Update` |
| combo points, combo target | — | rogue/druid bar |
| ammo id, farsight, action-bar toggles | — | the action bar |
| dodge / parry / block percentages | — | the character sheet |
| PvP rank and medal | — | the honor frame |
| hides helm / hides cloak | — | the paper doll |

Most want a plausible zero. Only the XP pair, the flags and the combo points move during a run.

## What is still broken

- **The rested tick, and what three builds proved it is not.** The standing error is
  `MainMenuBar.lua:25: attempt to compare number with nil` in `ExhaustionTick_Update`. Ruled out,
  each by a build and a log:
  - *Not* the XP pair being unfed: feeding `PLAYER_XP`/`PLAYER_NEXT_LEVEL_XP` from the run left the
    count at exactly two.
  - *Not* the load order: the stack is `[string "ExhaustionTick:OnEvent"]`, and the errors are
    timestamped **after** `in-game UI up`, so a handler runs them, not the build.
  - *Not* a bug in `GetXPExhaustion`: its nil is deliberate and follows the rest-state byte, as
    `ui_script/exp_bar_tests.rs` documents against the reference. Vanilla's own handler copes with
    nil on `PLAYER_ENTERING_WORLD` (`ui_script/action_bar_tests.rs:1921`).

  What is left is the event path: which event reaches `ExhaustionTick_Update` with a nil where the
  reference client would not, and whether `feed_units` fires it at a moment the reference never
  does. That needs the vanilla Lua read against the feed's event order — a sitting-down job, not
  another guess.
- **The action bar is empty, and that is the big one.** The stock bar reads action slots off the
  player's action fields. The mode has no spellbook and no action slots: its abilities live in
  `Run::abilities` and cast themselves on cooldown. Nothing short of giving the mode real action
  slots will fill that bar.
- **The screen is doubled.** The stock interface draws over the mode's own HUD — two health bars,
  two minimaps. Whatever of `survivors/hud.rs` the real frames replace should come out, and the
  level-up cards, which have no vanilla equivalent, should stay.

## What this cost, for the next person's estimate

Six builds, each about five minutes, each finding one true cause that was not sufficient on its
own. The gates came in a chain — the flag, then `SelfGuid`, then `SelfPlayer`, then the fields,
then the feed — and none was visible until the one before it was cleared. Budget for that shape
rather than for a fix.

## How to tell whether it is working

```bash
WOW_DATA=wow-data/Data ./target/release/warcraft-survivors.exe 2>&1 | tee run.log
grep -E "in-game UI up|runtime error" run.log
```

`ui_script: in-game UI up in … ms over … frame(s)` means FrameXML built. Any
`runtime error: Interface\…` line is a stock file finding the player wanting.
