
use std::time::Duration;

use bevy::prelude::*;
use bevy_tweening::Lens;

use crate::prelude::WorldCamera;

/// Various simple ad-hoc effects spawned by
/// components defined in this module.
pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_tweening::TweeningPlugin>() {
            app.add_plugins(bevy_tweening::TweeningPlugin);
        }
        app
            .add_systems(PostUpdate, shrink_and_disappear)
            .add_systems(PostUpdate, aim_for_camera)
        ;
    }
}

/// Marker for things that should shrink and disappear, at the given rate.
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct ShrinkAndDisappear {
    pub time: f32,
    func: EaseFunction,
    orig_scale: Option<Vec3>,
}

impl Default for ShrinkAndDisappear {
    fn default() -> Self {
        Self { time: 0.0, func: EaseFunction::Linear, orig_scale: None }
    }
}

impl ShrinkAndDisappear {
    pub fn new(time: Duration) -> Self {
        Self {
            time: time.as_secs_f32(),
            .. default()
        }
    }
    pub fn with_ease_function(self, func: EaseFunction) -> Self {
        Self {
            func,
            .. self
        }
    }
}

fn shrink_and_disappear(mut commands: Commands,
    time: Res<Time>,
    mut shrink_q: Query<(Entity, &mut ShrinkAndDisappear, &mut Transform)>
) {
    for (ent, mut sad, mut xfrm) in shrink_q.iter_mut() {
        if sad.orig_scale.is_none() {
            sad.orig_scale = Some(xfrm.scale);
        }

        let orig_scale = sad.orig_scale.as_ref().unwrap().clone();

        // Note, it's backwards, as we count down.
        let curve = EasingCurve::new(0.0, orig_scale.max_element(), sad.func);
        sad.time = (sad.time - time.delta_secs()).max(0.0);

        let new_scale_mag = curve.sample_clamped(sad.time);
        if new_scale_mag >= 0.01 {
            xfrm.scale = orig_scale * new_scale_mag;
        } else {
            commands.entity(ent).try_despawn();
        }
    }
}

/// Marker for things that should fly towards the camera.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct AimForCamera;

fn aim_for_camera(
    camera_q: Single<(Entity, &Transform), (With<WorldCamera>, Without<AimForCamera>)>,
    mut aim_q: Query<(Entity, &mut Transform, &GlobalTransform), With<AimForCamera>>
) {
    let (_cam_ent, cam_xfrm) = *camera_q;
    for (_ent, mut xfrm, _gxfrm) in aim_q.iter_mut() {
        xfrm.translation = xfrm.translation.lerp(cam_xfrm.translation, 0.25);
    }
}


#[derive(Debug)]
pub struct TransformPositionScaleLens {
    pub start: Transform,
    pub end: Transform,
}

impl Lens<Transform> for TransformPositionScaleLens {
    fn lerp(&mut self, mut target: Mut<Transform>, ratio: f32) {
        target.translation = self.start.translation.lerp(self.end.translation, ratio);
        target.scale = self.start.scale.lerp(self.end.scale, ratio);
    }
}

#[derive(Debug)]
pub struct TransformPositionRotationLens {
    pub start: Transform,
    pub end: Transform,
}

impl Lens<Transform> for TransformPositionRotationLens {
    fn lerp(&mut self, mut target: Mut<Transform>, ratio: f32) {
        target.translation = self.start.translation.lerp(self.end.translation, ratio);
        target.rotation = self.start.rotation.slerp(self.end.rotation, ratio);
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TextColorLens {
    /// Start color.
    pub start: Color,
    /// End color.
    pub end: Color,
}

impl Lens<TextColor> for TextColorLens {
    fn lerp(&mut self, mut target: Mut<TextColor>, ratio: f32) {
        target.0 = self.start.mix(&self.end, ratio);
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TextShadowColorLens {
    /// Start color.
    pub start: Color,
    /// End color.
    pub end: Color,
}

impl Lens<TextShadow> for TextShadowColorLens {
    fn lerp(&mut self, mut target: Mut<TextShadow>, ratio: f32) {
        target.color = self.start.mix(&self.end, ratio);
    }
}

#[derive(Debug)]
pub struct BackgroundColorLens {
    pub start: Color,
    pub end: Color,
}

impl Lens<BackgroundColor> for BackgroundColorLens {
    fn lerp(&mut self, mut target: Mut<BackgroundColor>, ratio: f32) {
        target.0 = self.start.mix(&self.end, ratio);
    }
}
