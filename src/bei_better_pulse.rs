//! Altered version of BEI 0.26 [`Pulse`] that tracks [`Instant`] to avoid [`Timer`] issues
//! caused by using the [`Fixed`] schedule, which can "fire" the same event multiple times
//! per tick.
//!
//! This is registered along with [`crate::ActionPlugin`].
use core::time::Duration;

use bevy::{platform::time::Instant, prelude::*};

use bevy_enhanced_input::prelude::*;

/// Returns [`TriggerState::Ongoing`] when input becomes actuated and [`TriggerState::Fired`]
/// on the defined time interval.
///
/// Note: [`Complete`] only fires when the repeat limit is reached or when input is released
/// immediately after being triggered. Otherwise, [`Cancel`] is fired when input is released.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Clone, Component, Debug)]
pub struct BetterPulse {
    /// Number of times the condition can be triggered (0 means no limit).
    pub trigger_limit: u32,

    /// Whether to trigger when the input first exceeds the actuation threshold or wait for the first interval.
    pub trigger_on_start: bool,

    /// Trigger threshold.
    pub actuation: f32,

    /// The type of time used to advance the timer.
    pub time_kind: TimeKind,

    /// Time in seconds that will be used instead of the [`Self::interval`] once.
    initial_delay: Option<f32>,

    /// Interval between pulses in seconds.
    interval: f32,

    last_fire_or_reset_time: Instant,

    trigger_count: u32,

    /// Tracks if we're in an actuated state to detect the start.
    started_actuation: bool,
}

impl BetterPulse {
    /// Creates a new instance with the given interval in seconds.
    #[must_use]
    pub fn new(interval: f32) -> Self {
        Self {
            trigger_limit: 0,
            trigger_on_start: true,
            actuation: 0.5 /* DEFAULT_ACTUATION */,
            time_kind: Default::default(),
            initial_delay: None,
            interval,
            last_fire_or_reset_time: Instant::now(),
            trigger_count: 0,
            started_actuation: false,
        }
    }

    #[must_use]
    pub fn with_trigger_limit(mut self, trigger_limit: u32) -> Self {
        self.trigger_limit = trigger_limit;
        self
    }

    #[must_use]
    pub fn trigger_on_start(mut self, trigger_on_start: bool) -> Self {
        self.trigger_on_start = trigger_on_start;
        self
    }

    /// Sets a different pause before the first repeat.
    ///
    /// Further repeats will use the interval from [`Self::new`].
    ///
    /// For example, you could set a longer delay to simulate keyboard repeat:
    /// when you hold down a key, the first repeat takes longer to fire, and then
    /// it repeats at a faster, steady interval.
    #[must_use]
    pub fn with_initial_delay(mut self, initial_delay: f32) -> Self {
        self.initial_delay = Some(initial_delay);
        self
    }

    /// Returns the delay from [`Self::with_initial_delay`] if it was set.
    #[must_use]
    pub fn initial_delay(&self) -> Option<f32> {
        self.initial_delay
    }

    #[must_use]
    pub fn with_actuation(mut self, actuation: f32) -> Self {
        self.actuation = actuation;
        self
    }

    #[must_use]
    pub fn with_time_kind(mut self, kind: TimeKind) -> Self {
        self.time_kind = kind;
        self
    }
}

impl InputCondition for BetterPulse {
    fn evaluate(
        &mut self,
        _actions: &ActionsQuery,
        time: &ContextTime,
        value: ActionValue,
    ) -> TriggerState {
        let now = Instant::now();
        if value.is_actuated(self.actuation) {
            let mut should_fire = false;

            if !self.started_actuation {
                self.started_actuation = true;
                should_fire |= self.trigger_on_start;
            }

            let pulse_time = Duration::from_secs_f32(self.initial_delay.unwrap_or(self.interval));

            let fire_time_elapsed = now.saturating_duration_since(self.last_fire_or_reset_time);

            // info!("pulse_time = {pulse_time:?}, fire_time_elapsed={fire_time_elapsed:?}");

            // Where is the time scaling? *shrug*
            // Anyway, if the time is zero, that means either (1) multiple fires per tick or (2) clock is paused.
            #[expect(deprecated)]
            let fire_time_elapsed = match self.time_kind {
                TimeKind::Real => if time.real.elapsed().is_zero() { Duration::ZERO } else { fire_time_elapsed }
                TimeKind::Auto |
                TimeKind::Virtual => if time.auto.elapsed().is_zero() { Duration::ZERO } else { fire_time_elapsed },
            };

            should_fire |= fire_time_elapsed >= pulse_time;

            if self.trigger_limit == 0 || self.trigger_count < self.trigger_limit {
                if should_fire {
                    self.trigger_count += 1;
                    self.last_fire_or_reset_time = now;
                    TriggerState::Fired
                } else {
                    TriggerState::Ongoing
                }
            } else {
                TriggerState::None
            }
        } else {
            self.trigger_count = 0;
            self.started_actuation = false;
            self.last_fire_or_reset_time = now;
            TriggerState::None
        }
    }
}
