//! The run's abilities on the stock action bar.
//!
//! The 1.12 bar does not read the player descriptor: `ActionButton_Update` asks `GetActionInfo`
//! and `GetActionTexture`, which answer out of the model's action table. So a mode with no
//! spellbook and no action fields can still fill the bar — [`UiScript::set_action`] names the
//! slot's icon and spell, [`UiScript::set_action_state`] carries the cooldown stock
//! `Cooldown.lua` sweeps. Slots are the first page, 1..=12, which is `ActionButton1`..`12`.
//!
//! The buttons are a display, not an input: the run's abilities cast themselves on cooldown
//! ([`super::abilities::cast_abilities`]), so a click's `UseAction` is left on the queue unread.

use bevy::prelude::*;

use benilla_ui::script::{ActionSlot, ActionState, ScriptValue, UiScript};

use super::Run;

/// `ActionBarFrame`'s first page; a run never owns more abilities than this.
const SLOTS: usize = 12;

/// Push the run's abilities into the action table, and their cooldowns every frame.
///
/// The slot set is memoised per VM: the icons move only when an ability is gained or the bar is
/// rebuilt, and `ACTIONBAR_SLOT_CHANGED` fires only then, as the reference's own feed does. A
/// fresh VM (a `ReloadUI`) re-pushes even when nothing changed.
pub(super) fn feed_action_bar(
    run: Option<Res<Run>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    script: Option<NonSendMut<UiScript>>,
    mut memo: Local<crate::ui_script::VmMemo<Vec<u32>>>,
) {
    let (Some(mut script), Some(run)) = (script, run) else {
        return;
    };

    // The spell in each slot, 0 for an empty one.
    let mut want = vec![0_u32; SLOTS];
    for (slot, owned) in run.abilities.iter().take(SLOTS).enumerate() {
        want[slot] = owned.def.spell;
    }

    let (seen, fresh_vm) = memo.get_reset(&script);
    if fresh_vm || *seen != want {
        *seen = want.clone();
        for (slot, &spell) in want.iter().enumerate() {
            let action = slot as u32 + 1;
            script.set_action(
                action,
                (spell != 0).then(|| ActionSlot {
                    texture: spells
                        .as_deref()
                        .and_then(|s| s.catalog.get(spell))
                        .and_then(|d| d.icon.clone()),
                    kind: 0,
                    action: spell,
                    count: 0,
                    consumable: false,
                }),
            );
            script.fire_event(
                "ACTIONBAR_SLOT_CHANGED",
                vec![ScriptValue::Int(i64::from(action))],
            );
        }
    }

    // The cooldown, re-pushed every frame: the start is absolute on the `GetTime` clock, so a
    // running sweep re-pushes the same triple and only a re-arm makes a new one.
    let stats = run.stats();
    let now = script.now();
    for slot in 0..SLOTS {
        let action = slot as u32 + 1;
        let Some(owned) = run.abilities.get(slot) else {
            script.set_action_state(action, None);
            continue;
        };
        let total = owned.cooldown(&stats).max(0.01);
        let remaining = owned.cd.clamp(0.0, total);
        let cooldown = (remaining > 0.0).then(|| {
            let started = now - f64::from(total - remaining);
            ((started * 1000.0) as i64, (total * 1000.0) as u32, true)
        });
        script.set_action_state(
            action,
            Some(ActionState {
                usable: true,
                cooldown,
                ..Default::default()
            }),
        );
    }
}
