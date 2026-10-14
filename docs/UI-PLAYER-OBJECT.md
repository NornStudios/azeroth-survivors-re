# Running survivors on the client's own interface

`main` draws the mode's own HUD; the game this is a build of draws the real 1.12 interface — unit
frames, action bar, minimap, chat — and that is most of why it reads as WoW. The engine under us
can do it: `benilla-ui` parses FrameXML and `benilla-app` carries the `ui_*` modules that feed it.
The mode just never switched it on.

This branch switches it on. The interface loads and binds, the player frame carries the run's own
health and level, the action bar carries its abilities, and the mode's duplicates of the frames
FrameXML draws are gone.

## The gates, in the order they appeared

Each of the first four was one line, and each was invisible until the one before it was fixed.

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
5. **The rested tick raised twice a run.** See below — the one that took the reading.

## `MainMenuBar.lua:25`, and why three experiments missed it

The standing error was `attempt to compare number with nil` in `ExhaustionTick_Update`. Extracted
from the install's own `patch.MPQ`, that line is:

```lua
22  local exhaustionThreshold = GetXPExhaustion();
23  local exhaustionStateID, exhaustionStateName, exhaustionStateMultiplier;
24  exhaustionStateID, exhaustionStateName, exhaustionStateMultiplier = GetRestState();
25  if (exhaustionStateID >= 3) then
```

The nil is **`GetRestState()`'s first return**, not `GetXPExhaustion()`. `GetXPExhaustion`'s nil is
handled three lines later at `if (not exhaustionThreshold)`, which is why the XP pair, the load
order and `GetXPExhaustion` itself all came back innocent: they were the wrong accessor.

The chain was already written down in the engine's own comments:

- `GetRestState()` answers `nil, nil, nil` when `model.exhaustion` holds no row for
  `model.rest_state` (`benilla-ui/script/unit/bindings.rs`).
- `model.rs` defaults `rest_state` to 2 deliberately, and names this exact failure: "a 0 has no
  Exhaustion row, and `GetRestState()`'s nil would raise at the unguarded `>= 3` of
  `MainMenuBar.lua:25`".
- `feed_units` pushes `player_rest_state().unwrap_or(0)`. The hero's descriptor never set
  `PLAYER_BYTES_2`, so that was `None` → 0, overwriting the deliberate 2.
- Byte 0 has no `Exhaustion.dbc` row — they run 1..5 — so the accessor returned nil.

Twice a run because two of the four events the handler guards on reach line 25: the first-sight
`UPDATE_EXHAUSTION` and `PLAYER_ENTERING_WORLD`.

**The fix belongs on the hero, not on the feed.** `benilla-protocol`'s accessor says so outright:
"Absent reads 0, which `GetRestState()` answers with nils; do not default it to 2, which would hide
a read before the create landed." The feed and the accessor are both right; what was wrong is that
the hero did not look like a character. `feed_player_frame` now writes `PLAYER_BYTES_2` (field 194)
with rest state 2, normal, which every 1.12 character carries.

An earlier carry in `ui_unit.rs` replaced upstream's `if let Some(store) = self_q…` guard with
zeros, on the premise that otherwise `UnitXPMax("player")` is nil. **That premise is false** —
`UnitXP`/`UnitXPMax` are `i64` bindings over `u32` model fields and cannot return nil — and the
rest-snapshot half of it is what forced the 0. Reverted; `ui_unit.rs` now carries nothing.

## The action bar

The bar does **not** read the player descriptor, whatever its reputation. `ActionButton_Update`
asks `GetActionInfo` and `GetActionTexture`, which answer out of the model's action table, fed by
`UiScript::set_action` and `set_action_state`. Those take a plain struct — no descriptor, no
spellbook, no server. Two places in the tree already pushed synthetic slots this way
(`addon_harness/render.rs`, `ui_script/mod.rs`).

So `survivors/actionbar.rs` is one system: the run's abilities into slots 1..=12 with their
`SpellIcon` art, `ACTIONBAR_SLOT_CHANGED` only when the set changes, and the cooldown re-pushed
every frame on the `GetTime` clock for stock `Cooldown.lua` to sweep. The buttons are a display,
not an input: abilities cast themselves on cooldown, so a click's `UseAction` is left unread.

It registers on its own, not in `survivors/mod.rs`'s chained tuple — inheriting that chain's
`after(show_panels)` closes a 13,000-cycle schedule error through `UiInput`.

## What came out of the mode's HUD

The mode's HUD was itself a hand-built Bevy replica of FrameXML pieces, so the overlaps were exact
and the removals are clean:

| gone | what replaces it |
| --- | --- |
| `XpBar`/`XpFill` and the four `UI-MainMenuBar-Dwarf` segment rules | stock `MainMenuExpBar` |
| `survivors/minimap.rs` entire (~500 lines, 3 systems) | stock `MinimapCluster` |
| `AbilityRow` and its cooldown sweeps | the stock action bar |

Kept, having no vanilla counterpart: the run clock with its level and kill line, the passive row
(moved 100 units down, clear of `PlayerFrame`), the mobility button, the banner, the level-up
cards, pause and the results screen. The hero's floating health bar stays too — it is a
`CGNamePlateFrame` over the character, not a second `PlayerFrame`.

## Still open

- **The minimap no longer dots enemies.** `survivors/minimap.rs`'s `compose` drew every live
  `Enemy` as a red blip; the engine's blip layer wants `NetEntity` and the reference's tracking
  rules, so the stock map shows none. Restoring the radar means feeding the mode's enemies through
  `minimap/blips`, or emitting a survivors blip layer into the stock `Minimap` frame.
- **The chat frame renders as an opaque white box.** Its buttons are right and its backdrop is
  not. Pre-existing, on the stock FrameXML path, and unrelated to anything here;
  `FloatingChatFrame.xml`'s `ChatFrameEditBox: SetScript("OnInputLanguageChanged") unsupported in
  v1` is the one warning the load still prints and the first thing to check.

## How to tell whether it is working

```bash
WOW_DATA=/c/NornStudios/azeroth-survivors-re/wow-data/Data \
WOW_SURVIVORS_AUTO=warrior:all WOW_SURVIVORS_SHOTS=shots \
  ./target/release/warcraft-survivors.exe > run.log 2>&1
grep -E "in-game UI up|runtime error" run.log
```

`WOW_SURVIVORS_AUTO=<class>[:idle|:all|:late]` plays the run by itself and `WOW_SURVIVORS_SHOTS`
writes PNGs of it, so this is checkable without a person at the keyboard.
`ui_script: in-game UI up in … ms over … frame(s)` means FrameXML built. The only
`runtime error:` line a good run prints is the `OnInputLanguageChanged` one above.
