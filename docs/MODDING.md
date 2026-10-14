# Changing the game

Everything the *survivors mode* is lives in **`crates/benilla-app/src/survivors/`** — about 9 000
lines across thirteen files. The twenty `benilla-*` crates around it are a from-scratch WoW 1.12.1
client (MPQ, BLP, M2, DBC, terrain, UI engine); you almost never need to touch them to change the
game. `crates/warcraft-survivors` is a fifteen-line launcher.

| file | what it owns |
| --- | --- |
| `data.rs` | **the tables**: abilities, passives, classes, enemies, bosses, maps, the wave clock |
| `abilities.rs` | auto-cast: what each `Kind` does when its cooldown comes up, and how a hit lands |
| `mod.rs` | the run: owned abilities, rank scaling, the hero's derived `Stats` |
| `cards.rs` | the level-up picker: which three cards are offered, their rarity and their text |
| `enemies.rs` | spawning, chasing, death, loot |
| `hero.rs` | the avatar, movement, health, experience |
| `mobility.rs` | the class movement spell on **F** |
| `hud.rs`, `menu.rs`, `minimap.rs`, `grade.rs` | the pick screen, the in-run HUD, the results screen |
| `units.rs`, `auto.rs` | virtual units for the engine, and the visual effects bridge |

A spell id in these tables (`spell: 133`) is a **`Spell.dbc` row, used only for its name, icon,
visual chain and missile speed**. Every number that matters — damage, cooldown, range — is the
mode's own. Nothing is read from the client's spell data, so a spell can behave however you like
while wearing Fireball's art.

## Adding a spell

**1. Define it** in `ABILITIES` (`survivors/data.rs:125`). The `ability!` macro fills everything
you leave out from `ab()` (`data.rs:100`): cooldown 2 s, range 20 yd, count 1, no effect.

```rust
ability!(
    "frost_nova_burst",     // the key: unique, and the fallback display name
    122,                    // Spell.dbc row — icon, name, cast and impact visuals, missile speed
    Nova {
        damage: 30.0,
        cooldown: 6.0,
        radius: 12.0,
        effect: Effect::Root(2.0),
        mark: 122           // the spell whose state visual marks an affected enemy
    }
),
```

**2. Give it to a class** — add the key to that class's `pool` in `CLASSES` (`data.rs:989`), or
make it the class's `start` spell. A class offers only what is in its pool; a key in `ABILITIES`
that no pool names is dead weight.

**3. Build and play.** Nothing else is wired by hand: the card deck reads `ABILITIES`, ranks it
against the class's other spells with `power()` (`cards.rs:60`) to colour its rarity, and the HUD
pulls the icon and name from `Spell.dbc` through the same id.

### The kinds

`Kind` (`data.rs:25`) is the whole behaviour catalogue — a new spell is a new *row*, not new code,
as long as one of these fits:

| kind | fires |
| --- | --- |
| `Bolt` | a missile at the nearest enemies (instant if the spell has no missile speed) |
| `Strike` / `Melee` | an instant hit at range / in melee reach |
| `Nova` / `Cone` | everything around the hero / in a cone toward the nearest enemy |
| `Ground` / `GroundSelf` | a ticking area at the densest crowd / under the hero |
| `Chain` | jumps enemy to enemy |
| `Dot` | a damage-over-time on several enemies |
| `Aura` / `Orbit` | always on: pulses around the hero / zaps one enemy, every `cooldown` |
| `Shield` / `Immune` / `Thorns` / `Heal` | the self kinds, for `duration` |
| `Totem` | drops a totem that casts `totem.1` for `duration` |

`Effect` (`data.rs:61`) rides along with any damaging kind: `Slow`, `Root`, `Stun`, `Fear`,
`Weaken`, `Drain` (lifesteal) and `Execute` (kill under a health fraction). A genuinely new
*behaviour* means a new `Kind` arm in the `match def.kind` in `abilities.rs:252` — that is the one
place casting dispatches.

### What a rank up does

Ranks are global, not per spell (`mod.rs:201`):

- damage ×`1 + 0.3·(rank−1)`
- cooldown ×`0.93^(rank−1)`, floored at 0.2 s after haste
- `+1` target at ranks 3, 5 and 7
- radius ×`1 + 0.08·(rank−1)`, duration ×`1 + 0.1·(rank−1)`

Caps: `MAX_ABILITY_LEVEL = 8`, `MAX_ABILITIES = 6`, `MAX_PASSIVES = 6` (`data.rs:888`).

## Adding a passive

`PASSIVES` (`data.rs:916`) is a key, a name, an icon spell, a `Stat` and a per-level amount. The
ten stats are `Might, MaxHp, Speed, Regen, Haste, Area, Magnet, Growth, Armor, Crit`; they are
summed into the hero's `Stats` in `Run::stats` (`mod.rs:310`), which is also where a **new** stat
would need its arm. Passives are offered to every class.

## The knobs worth turning first

| what | where |
| --- | --- |
| run length (survive to win) | `VICTORY_SECS = 900.0` — `data.rs:1309` |
| boss cadence | `BOSS_EVERY_SECS = 135.0` — `data.rs:1311` |
| experience per gem | `XP_RATE = 2.0` — `data.rs:1313` |
| level curve | `xp_to_next()` — `data.rs:1346` |
| crowd size over the run | `target_alive()` — `data.rs:1331` (14 → 220) |
| enemy scaling | `hp_scale()`, `damage_scale()` — `data.rs:1336` |
| which tier spawns when | `wave_pool()` — `data.rs:1321` |
| hero base | `BASE_HP 120`, `BASE_SPEED 7`, `BASE_MAGNET 4.5` — `mod.rs:235`; per-class bonus at `mod.rs:324` |
| base crit | `crit: 0.05` in `Run::stats` — `mod.rs:321` |
| the F spell, per class | `MOBILITY` — `mobility.rs:43`; learned at `LEARN_LEVEL = 5` |

Enemies, bosses and battlegrounds are tables too: `ENEMIES` (`data.rs:1239`) and `BOSSES`
(`data.rs:1281`) are The Barrens' hand-tuned ladder; every other zone is a `MapDef` in `MAPS`
(`data.rs:1407`) whose `foes`/`bosses` are names, display ids, levels, speeds and scales, with
health and damage read off the Barrens ladder by roster position.

## Finding a spell id

Any 1.12.1 `Spell.dbc` row works. To see what one looks like before you use it:

```bash
cargo run --release -p benilla-formats --bin benilla-extract -- wow-data/Data spellvis 133
```

It prints the spell's name, missile speed and every visual stage. A row with no `SpellVisual` is a
silent cast — fine for an `Aura`, wrong for a `Bolt`. To browse ids, dump the table:

```bash
cargo run --release -p benilla-formats --bin benilla-extract -- wow-data/Data dbc DBFilesClient/Spell.dbc spell.csv
```

## The loop

The browser build is the slow one (~18 min cold, and the page reloads 77 MB of wasm). For tuning
tables, the **native build** is the one to iterate on:

```bash
WOW_DATA=wow-data/Data cargo run --release -p warcraft-survivors
```

`cargo check -p benilla-app` catches a table mistake in seconds without either build.
