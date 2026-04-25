//! This module manages client synth listening.
//!

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::time::Duration;

use bevy::ecs::entity::EntityHashMap;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::*;

use crate::midi_synth::synth::*;

use crate::synth::MidiSynthProxy;
use crate::synth::SynthChannel;
use crate::synth::SynthClock;
use crate::synth::SynthCommand;
use crate::synth::SynthMessage;
use crate::synth::SynthNote;

pub struct ClientSynthPlugin;

impl Plugin for ClientSynthPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<PendingSynthEvents>()
            .register_type::<SynthController>()
            .register_type::<SynthProxyMap>()
            .init_resource::<PendingSynthEvents>()
            .init_resource::<SynthController>()
            .init_resource::<SynthProxyMap>()

            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::Initializing)
                    .load_collection::<CommonSoundFontAssets>()
            )
            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::LoadingSave)
                    .load_collection::<CommonSoundFontAssets>()
            )

            // Reset when we know no gameplay should be active.
            // Other clients should use DespawnOnExit for MidiSynth.
            .add_systems(OnExit(GameplayState::Playing),
                reset_synth_config,
            )
            .add_systems(FixedUpdate,
                (
                    handle_synth_events,
                    cleanup_synths,
                )
                .run_if(not(is_paused))
            )
        ;
    }
}

#[derive(Resource, Clone, PartialEq, Reflect)]
#[reflect(Resource, Clone)]
#[type_path = "game"]
pub struct SynthController {
    /// Number of synths to spawn.
    pub max_midi_synths: u8,
}

impl Default for SynthController {
    fn default() -> Self {
        Self {
            max_midi_synths: 8,
        }
    }
}

#[derive(Resource, Clone, Default, PartialEq, Reflect)]
#[reflect(Resource, Clone)]
#[type_path = "game"]
pub struct SynthProxyMap {
    pub map: EntityHashMap<Entity>,
    pub synths: EntityHashSet,
}
impl SynthProxyMap {
    pub fn register_synth(&mut self, entity: Entity) {
        self.map.insert(entity, entity);
        self.synths.insert(entity);
    }
}

/// Lists of events pending.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
struct PendingSynthEvents(VecDeque<(Duration, SynthMessage)>);

fn reset_synth_config(
    mut commands: Commands,
    mut synth_q: Query<(Entity, &mut MidiSynth)>,
    mut synth_map: ResMut<SynthProxyMap>,
) {
    for (ent, synth) in synth_q.iter_mut() {
        synth.thread_quit.store(true, Ordering::Release);
        commands.entity(ent).remove::<MidiSynth>();
    }

    synth_map.map.clear();
    synth_map.synths.clear();
}

fn handle_synth_events(
    mut reader: MessageReader<SynthMessage>,
    synth_map: Res<SynthProxyMap>,
    mut synth_q: Query<&mut MidiSynth>,
    mut waiting: ResMut<PendingSynthEvents>,
    time: Res<Time>,
    mut clock: ResMut<SynthClock>,
) {
    let waiting = &mut waiting.0;

    // Queue timed events.
    // We store these as relative countdowns due to variable time rates.
    // (We can't set a deadline since we don't know if time will be paused.)
    let mut need_sort = false;
    let mut need_reset = false;
    for event in reader.read() {
        let SynthMessage(_, _, delay) = event;
        let delay = *delay + clock.delay_to_next();
        // See if we're adding anything ahead of schedule.
        if let Some(last) = waiting.back() && delay < last.0 {
            need_sort = true;
        }
        if matches!(event, SynthMessage(_, SynthCommand::Reset, _)) {
            need_reset = true;
            need_sort = true;
        }
        waiting.push_back((delay, event.clone()));
    }

    if need_sort {
        waiting.make_contiguous().sort_by(|a, b| {
            if a.0 == b.0 && matches!(a.1, SynthMessage(_, SynthCommand::Reset, _)) {
                return std::cmp::Ordering::Less
            }
            a.0.cmp(&b.0)
        });
    }
    if need_reset {
        while let Some((_, event)) = waiting.front() {
            if matches!(event, SynthMessage(_, SynthCommand::Reset, _)) {
                break;
            }
            waiting.pop_front();
        }
    }

    while let Some((delay, event)) = waiting.front_mut() {
        *delay = delay.saturating_sub(time.delta());
        if delay.is_zero() {
            let target = synth_map.map.get(&event.0).unwrap_or(&event.0);
            let target = *target;
            if let Ok(mut synth) = synth_q.get_mut(target) {
                send_synth_command(&event.1, &mut synth);
                let _ = waiting.pop_front();
            } else {
                error!("no MidiSynth component for entity {}", target);
                let _ = waiting.drain(..);
            }
        } else {
            break
        }
    }

    clock.tick(time.delta());
}

fn send_synth_command(
    command: &SynthCommand,
    synth: &mut MidiSynth,
) {
    let to_channel = |c: &SynthChannel| match c {
        SynthChannel::Voice(v) => {
            let c = *v & 15;
            if c == 9 { 15 } else { c }
        }
        SynthChannel::Drums(_d) => 9,
    };
    let to_data = |v: &f32| (v * 127.0).clamp(0.0, 127.0) as u8;
    let to_key = |n: &SynthNote| n.to_midi();

    match command {
        SynthCommand::NoteOn(channel, note, vel) => synth.handle_event(
            MidiMessage::NoteOn{ channel: to_channel(channel), note: to_key(note), velocity: to_data(vel) },
        ),
        SynthCommand::NoteOff(channel, note) => synth.handle_event(
            MidiMessage::NoteOff{ channel: to_channel(channel), note: to_key(note) },
        ),
        SynthCommand::ChannelOff(channel) => synth.handle_event(
            MidiMessage::Controller {
                channel: to_channel(channel),
                ctrl: MidiMessage::CTRL_ALL_SOUNDS_OFF,
                data: 0,
            }
        ),
        SynthCommand::ProgramChange(channel, program) => synth.handle_event(
            MidiMessage::SetPatch{ channel: to_channel(channel), patch: *program }
        ),
        SynthCommand::ChannelVolume(channel, volume) => synth.handle_event(
            MidiMessage::Controller{
                channel: to_channel(channel),
                ctrl: MidiMessage::CTRL_SET_VOLUME_COARSE,
                data: to_data(volume),
            }
        ),
        SynthCommand::Reset => {
            for channel in 0..16 {
                synth.handle_event(
                    MidiMessage::Controller {
                        channel,
                        ctrl: MidiMessage::CTRL_ALL_SOUNDS_OFF,
                        data: 0,
                    }
                );
            }
        },
    }
}

fn cleanup_synths(
    mut removed_proxies: RemovedComponents<MidiSynthProxy>,
    mut removed_synths: RemovedComponents<MidiSynth>,
    mut waiting: ResMut<PendingSynthEvents>,
    mut synth_map: ResMut<SynthProxyMap>,
) {
    for ent in removed_proxies.read() {
        synth_map.map.remove(&ent);

        waiting.0.retain_mut(|(_, event)| {
            !matches!(event, SynthMessage(sent, _, _) if *sent == ent)
        });
    }
    for ent in removed_synths.read() {
        synth_map.synths.remove(&ent);
    }
}
