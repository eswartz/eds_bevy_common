use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;

use std::collections::VecDeque;
use std::time::Duration;

use sysinfo;

use crate::physics::*;

use crate::Player;
use crate::PlayerLook;
use crate::PlayerMovement;
use crate::ProgramState;
use crate::repeating_with_delay;

pub struct StatsOverlayPlugin;

impl Plugin for StatsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(StatsOverlayVisible(false))
            .init_resource::<StatsOverlayStyle>()
            .init_resource::<StatsRegistry>()
            .init_resource::<DeltaBuffer>()
            .init_resource::<SysInfoBuffer>()

            .add_systems(
                Startup,
                add_default_providers,
            )
            .add_systems(
                OnEnter(ProgramState::LaunchMenu),
                    update_stats_visibility
            )
            .add_systems(
                Update,
                    update_stats_visibility
                        .run_if(resource_changed::<StatsOverlayVisible>)
            )
            .add_systems(
                Update,
                (
                    refresh_sys_info.run_if(repeating_with_delay(Duration::from_secs_f32(1.0 / 15.0))),
                    refresh_fps_info,
                    diagnostic_system,
                )
            )
        ;
    }
}

/// This is provided by [StatsProvider::fetch_value] and
/// provides the text and importance flag for a value.
/// (Importance currently means "error")
#[derive(Debug, Default, Clone, PartialEq)]
pub struct StatsValue {
    /// Display text.
    pub label: String,
    /// If true, value needs attention in UI.
    pub important: bool,
}

impl StatsValue {
    pub fn new(label: impl Into<String>) -> Self {
        let text = label.into();
        Self { label: text, important: false }
    }
    pub fn with_importance(self, important: bool) -> Self {
        Self {
            important,
            ..self
        }
    }
}

/// Implement this to add data to the stats display.
pub trait StatsProvider: Send + Sync + 'static {
    /// Get the displayed label.
    fn get_label(&self) -> String;
    /// Compute the value string and importance.
    fn fetch_value(&self, world: &mut World) -> StatsValue;
    /// Override sort order.
    fn priority(&self) -> i32 { 0 }
}

/// This organizes all the stats providers.
#[derive(Resource, Default)]
pub struct StatsRegistry {
    pub items: Vec<Box<dyn StatsProvider>>,
}

impl StatsRegistry {
    pub fn add_provider(&mut self, provider: Box<dyn StatsProvider>) {
        self.items.push(provider);

        self.items.sort_by_key(|a| a.priority());
    }
    pub fn reset_providers(&mut self) {
        self.items.clear();
    }

    pub fn providers(&self) -> &Vec<Box<dyn StatsProvider>> {
        &self.items
    }
    pub fn providers_mut(&mut self) -> &mut Vec<Box<dyn StatsProvider>> {
        &mut self.items
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
}


const DELTA_BUFFER_LEN: usize = 16;

/// Track the .delta counts from the last [DELTA_BUFFER_LEN] frames.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct DeltaBuffer(pub VecDeque<Duration>);

pub struct FpsProvider;

impl StatsProvider for FpsProvider {
    fn get_label(&self) -> String {
        "FPS".to_string()
    }
    fn priority(&self) -> i32 { -10 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        if let Some(time_buffer) = world.get_resource::<DeltaBuffer>() {
            // fps = time_buffer.0.len() as f32 / time_buffer.0.iter().sum::<f32>();
            let mut time_it = time_buffer.0.iter();
            let Some(time) = time_it.next() else {
                return StatsValue::new("???");
            };
            let mut total_time = time.as_secs_f32();
            // Each successive time is less relevant.
            let mut total = 1;
            for (index, time) in time_it.enumerate() {
                total_time += time.as_secs_f32() * index as f32;
                total += index;
            }
            let fps = total as f32 / total_time;
            StatsValue::new(format!("{:.0}", if fps.is_infinite() { 0.0 } else { fps }))
        } else {
            StatsValue::new("???")
        }
    }
}

pub struct FpsMaxProvider;

impl StatsProvider for FpsMaxProvider {
    fn get_label(&self) -> String {
        "Max Frame".to_string()
    }
    fn priority(&self) -> i32 { -9 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        if let Some(time_buffer) = world.get_resource::<DeltaBuffer>() {
            let max_ft = time_buffer.0.iter().max_by(|a, b|
                a.partial_cmp(b).unwrap_or(::core::cmp::Ordering::Equal)
                ).unwrap_or(&Duration::ZERO);
            StatsValue::new(format!("{:.2?}", max_ft))
        } else {
            StatsValue::new("???")
        }
    }
}

pub struct EntCountProvider;

impl StatsProvider for EntCountProvider {
    fn get_label(&self) -> String {
        "Entities".to_string()
    }

    fn priority(&self) -> i32 { -8 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        let count = world.entities().count_spawned() as usize;
        StatsValue::new(format!("{count}"))
    }
}

pub struct ContactCountProvider;

impl StatsProvider for ContactCountProvider {
    fn get_label(&self) -> String {
        "Contacts".to_string()
    }

    fn priority(&self) -> i32 { -7 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        if let Some(solver_diags) = world.get_resource::<SolverDiagnostics>() {
            StatsValue::new(format!("{}", solver_diags.contact_constraint_count))
        } else {
            StatsValue::new("???")
        }
    }
}

/// Storage for the [sysinfo::System] singleton and
/// a timer for updating it.
#[derive(Resource, Default)]
struct SysInfoBuffer(pub sysinfo::System, pub Timer);

fn refresh_fps_info(mut time_buffer: ResMut<DeltaBuffer>, time: Res<Time>) {
    let delta = time.delta();

    time_buffer.0.push_back(delta);
    while time_buffer.0.len() > DELTA_BUFFER_LEN {
        let _ = time_buffer.0.pop_front();
    }
}

fn refresh_sys_info(mut buffer: ResMut<SysInfoBuffer>, time: Res<Time>) {
    let delta = time.delta();

    if buffer.1.duration().is_zero() {
        buffer.1 = Timer::new(Duration::from_secs_f32(1.0 / 10.0), TimerMode::Repeating);
    }
    if buffer.1.tick(delta).just_finished() {
        buffer.0.refresh_cpu_usage();
        buffer.0.refresh_memory();
    }
}

#[derive(Default)]
pub struct CpuUsageProvider;

impl StatsProvider for CpuUsageProvider {
    fn get_label(&self) -> String {
        "CPU Usage".to_string()
    }

    fn priority(&self) -> i32 { -6 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
         if let Some(info) = world.get_resource::<SysInfoBuffer>() {
            StatsValue::new(format!("{}%", info.0.global_cpu_usage() as i32))
         } else {
            StatsValue::default()
         }
    }
}

#[derive(Default)]
pub struct MemoryUsageProvider;

impl StatsProvider for MemoryUsageProvider {
    fn get_label(&self) -> String {
        "Memory Usage".to_string()
    }

    fn priority(&self) -> i32 { -5 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        if let Some(sys_info) = world.get_resource::<SysInfoBuffer>() {
            let pct = (sys_info.0.used_memory() * 100).checked_div(sys_info.0.total_memory()).unwrap_or(0);
            StatsValue::new(format!("{}%", pct))
         } else {
            StatsValue::default()
         }
    }
}

#[derive(Default)]
pub struct PlayerPosProvider;

impl StatsProvider for PlayerPosProvider {
    fn get_label(&self) -> String {
        "Player Pos".to_string()
    }

    fn priority(&self) -> i32 { -4 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        let mut xfrm_q = world.query_filtered::<&Transform, With<Player>>();
        if let Some(xfrm) = xfrm_q.iter(world).next() {
            StatsValue::new(format!("[{:.1?},{:.1?},{:.1?}]",
                xfrm.translation.x,
                xfrm.translation.y,
                xfrm.translation.z,
            ))
        } else {
            default()
        }
    }
}


#[derive(Default)]
pub struct PlayerAngProvider;

impl StatsProvider for PlayerAngProvider {
    fn get_label(&self) -> String {
        "Player Look".to_string()
    }

    fn priority(&self) -> i32 { -4 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        let mut look_q = world.query_filtered::<&PlayerLook, With<Player>>();
        if let Some(look) = look_q.iter(world).next() {
            let (y, x, _) = look.rotation.to_euler(EulerRot::default());
            StatsValue::new(format!("{:.1?} / {:.1?}", y.to_degrees(), x.to_degrees()))
        } else {
            default()
        }
    }
}

fn add_default_providers(mut regy: ResMut<StatsRegistry>) {
    regy.add_provider(Box::new(FpsProvider));
    regy.add_provider(Box::new(FpsMaxProvider));
    regy.add_provider(Box::new(EntCountProvider));
    regy.add_provider(Box::new(ContactCountProvider));
    regy.add_provider(Box::new(CpuUsageProvider));
    regy.add_provider(Box::new(MemoryUsageProvider));
    regy.add_provider(Box::new(PlayerPosProvider));
    regy.add_provider(Box::new(PlayerAngProvider));
    regy.add_provider(Box::new(PlayerMoveProvider));
}


#[derive(Default)]
pub struct PlayerMoveProvider;

impl StatsProvider for PlayerMoveProvider {
    fn get_label(&self) -> String {
        "Player State/Move".to_string()
    }

    fn priority(&self) -> i32 { -4 }

    fn fetch_value(&self, world: &mut World) -> StatsValue {
        let mut query = world.query_filtered::<(&PlayerMovement, &LinearVelocity), With<Player>>();
        if let Some((movement, vel)) = query.iter(world).next() {
            StatsValue::new(format!("{:?}|{:.3} m/s|{:?}",
                movement.state,
                vel.0.xy().length(),
                movement.area,
            ))

        } else {
            default()
        }
    }
}

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct StatsOverlayVisible(pub bool);

/// This marks the UI node.
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct StatsOverlayMarker;

fn update_stats_visibility(
    // mut commands: Commands,
    visible: Res<StatsOverlayVisible>,
    mut marker_vis_q: Query<(&StatsOverlayMarker, &mut Visibility)>,
) {
    let new_vis = if !visible.0 { Visibility::Hidden } else { Visibility::Inherited };
    for (_, mut vis) in marker_vis_q.iter_mut() {
        *vis = new_vis;
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct StatsOverlayStyle {
    pub outer_margin: f32,
    pub inner_margin: f32,
    pub font_size: f32,
    pub font: Handle<Font>,
}
impl Default for StatsOverlayStyle {
    fn default() -> Self {
        Self {
            outer_margin: 4.,
            inner_margin: 4.,
            font_size: 10.,
            font: default(),
        }
    }
}

const PLAIN_COLOR: Color = Color::Srgba(bevy::color::palettes::tailwind::GRAY_50);
const IMPORTANT_COLOR: Color = Color::Srgba(bevy::color::palettes::tailwind::RED_500);

fn diagnostic_system(
    world: &mut World,
    mut refresh_timer: Local<f32>,

    mut cached: Local<::std::cell::OnceCell<Vec<Entity>>>,
) {
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);

    {
        let Some(stats_registry) = world.get_resource::<StatsRegistry>() else { return };
        let Some(style) = world.get_resource::<StatsOverlayStyle>() else { return };
        let Some(time) = world.get_resource::<Time>() else { return };

        // Fetch the [Entity]s for the [Text] nodes to edit.
        if let Some(prev_ents) = cached.get() && prev_ents.len() != stats_registry.len() {
            log::warn!("resetting {} vs {}", prev_ents.len(), stats_registry.len());
            let _ = cached.take();
        }
        let text_ents = cached.get_or_init(|| {
            // Generate the UI once.

            let mut result  = Vec::with_capacity(stats_registry.len());
            let font = TextFont {
                font: style.font.clone().into(),
                font_size: FontSize::Px(style.font_size),
                ..default()
            };
            commands.spawn((
                StatsOverlayMarker,
                Node {
                    margin: UiRect {
                        right: Val::Auto,
                        left: Val::Px(style.outer_margin),
                        top: Val::Px(style.outer_margin),
                        bottom: Val::Auto
                    },
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.5)),
                Visibility::Hidden,  // updated in `update_stats_visibility`
            )).with_children(|c| {
                c.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect {
                        left: Val::Px(style.inner_margin),
                        right: Val::Px(style.inner_margin),
                        top: Val::Px(style.inner_margin),
                        bottom: Val::Px(style.inner_margin)
                    },
                    align_items: AlignItems::FlexStart,
                    ..Default::default()
                }).with_children(|c| {
                    stats_registry
                        .providers()
                        .iter()
                        .for_each(|prov|
                            result.push(c.spawn((Node::default(), font.clone(), Text::new(prov.get_label()))).id())
                        );
                });
                c.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect {
                        left: Val::Px(style.inner_margin),
                        right: Val::Px(style.inner_margin),
                        top: Val::Px(style.inner_margin),
                        bottom: Val::Px(style.inner_margin)
                    },
                    min_width: Val::Px(80.),
                    align_items: AlignItems::FlexEnd,
                    ..Default::default()
                }).with_children(|c| {
                    result.clear();
                    stats_registry
                        .providers()
                        .iter()
                        .for_each(|_provider| result.push(c.spawn((
                            Node::default(),
                            font.clone(),
                            Text::default(),
                        )).id()
                    ));
                });
            });

            result
        }).clone();

        *refresh_timer += time.delta_secs();
        if *refresh_timer > 0.05 {
            *refresh_timer = 0.;

            world.resource_scope::<StatsRegistry, ()>(|world, stats_registry| {
                // let Some(stats_registry) = world.get_resource::<StatsRegistry>() else { return };
                let values = stats_registry
                    .providers()
                    .iter()
                    .map(|prov| prov.fetch_value(world))
                    .collect::<Vec<_>>();

                let mut text_color = world.query::<(&mut Text, &mut TextColor)>();
                for (index, value) in values.into_iter().enumerate() {
                    #[expect(clippy::indexing_slicing, reason = "if fails, bug in iter")]
                    if let Ok((mut text, mut color)) = text_color.get_mut(world, text_ents[index]) {
                        text.clear();
                        **text = value.label;
                        **color = if value.important { IMPORTANT_COLOR } else { PLAIN_COLOR };
                    }
                }
            });
        }

    }
    queue.apply(world);
}
