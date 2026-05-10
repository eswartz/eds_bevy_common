use bevy::ecs::world::CommandQueue;
/// eswartz: Based on `bevy_mini_fps` (single-file implementation in `lib.rs`).

/// I had some build problems and also wanted the
/// Plugin model, so the interface is totally different.
use bevy::prelude::*;

use bevy::time::common_conditions::repeating_after_delay;
use sysinfo;
use std::collections::VecDeque;
use std::time::Duration;

use avian3d::dynamics::solver::SolverDiagnostics;

use crate::Player;
use crate::PlayerLook;
use crate::ProgramState;

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
                    refresh_sys_info.run_if(repeating_after_delay(Duration::from_secs_f32(1.0 / 15.0))),
                    refresh_fps_info,
                    diagnostic_system,
                )
            )
        ;
    }
}

/// Implement this to add data to the stats display.
pub trait StatsProvider: Send + Sync + 'static {
    /// Get the displayed label.
    fn get_label(&self) -> String;
    /// Compute the value string.
    fn format_value(&self, world: &mut World) -> String;
    /// Tell if the stat is important (needs highlighting).
    /// This is only checked once and is used to construct the UI.
    fn is_important(&self) -> bool { false }
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

        self.items.sort_by(|a, b| a.priority().cmp(&b.priority()));
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

    fn format_value(&self, world: &mut World) -> String {
        if let Some(time_buffer) = world.get_resource::<DeltaBuffer>() {
            // fps = time_buffer.0.len() as f32 / time_buffer.0.iter().sum::<f32>();
            let mut time_it = time_buffer.0.iter();
            let Some(time) = time_it.next() else {
                return "???".to_string();
            };
            let mut total_time = time.as_secs_f32();
            // Each successive time is less relevant.
            let mut total = 1;
            for (index, time) in time_it.enumerate() {
                total_time = total_time + time.as_secs_f32() * index as f32;
                total += index;
            }
            let fps = total as f32 / total_time;
            format!("{:.0}", if fps.is_infinite() { 0.0 } else { fps })
        } else {
            "???".to_string()
        }
    }
}

pub struct FpsMaxProvider;

impl StatsProvider for FpsMaxProvider {
    fn get_label(&self) -> String {
        "Max Frame".to_string()
    }
    fn priority(&self) -> i32 { -9 }

    fn format_value(&self, world: &mut World) -> String {
        if let Some(time_buffer) = world.get_resource::<DeltaBuffer>() {
            let max_ft = time_buffer.0.iter().max_by(|a, b|
                a.partial_cmp(b).unwrap_or(::core::cmp::Ordering::Equal)
                ).unwrap_or(&Duration::ZERO);
            format!("{:.2?}", max_ft)
        } else {
            "???".to_string()
        }
    }
}

pub struct EntCountProvider;

impl StatsProvider for EntCountProvider {
    fn get_label(&self) -> String {
        "Entities".to_string()
    }

    fn priority(&self) -> i32 { -8 }

    fn format_value(&self, world: &mut World) -> String {
        let count = world.entities().count_spawned() as usize;
        format!("{count}")
    }
}

pub struct ContactCountProvider;

impl StatsProvider for ContactCountProvider {
    fn get_label(&self) -> String {
        "Contacts".to_string()
    }

    fn priority(&self) -> i32 { -7 }

    fn format_value(&self, world: &mut World) -> String {
        if let Some(solver_diags) = world.get_resource::<SolverDiagnostics>() {
            format!("{}", solver_diags.contact_constraint_count)
        } else {
            "???".to_string()
        }
    }
}


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

    fn format_value(&self, world: &mut World) -> String {
         if let Some(info) = world.get_resource::<SysInfoBuffer>() {
            format!("{}%", info.0.global_cpu_usage() as i32)
         } else {
            String::new()
         }
    }
}

#[derive(Default)]
pub struct MemoryUsageProvider {
}

impl MemoryUsageProvider {
}

impl StatsProvider for MemoryUsageProvider {
    fn get_label(&self) -> String {
        "Memory Usage".to_string()
    }

    fn priority(&self) -> i32 { -5 }

    fn format_value(&self, world: &mut World) -> String {
        if let Some(sys_info) = world.get_resource::<SysInfoBuffer>() {
            let pct = (sys_info.0.used_memory() * 100).checked_div(sys_info.0.total_memory()).unwrap_or(0);
            format!("{}%", pct)
         } else {
            String::new()
         }
    }
}

#[derive(Default)]
pub struct PlayerPosProvider {
}

impl PlayerPosProvider {
}

impl StatsProvider for PlayerPosProvider {
    fn get_label(&self) -> String {
        "Player Pos".to_string()
    }

    fn priority(&self) -> i32 { -4 }

    fn format_value(&self, world: &mut World) -> String {
        let mut xfrm_q = world.query_filtered::<&Transform, With<Player>>();
        for xfrm in xfrm_q.iter(world) {
            return format!("[{:.1?},{:.1?},{:.1?}]",
                xfrm.translation.x,
                xfrm.translation.y,
                xfrm.translation.z,
            );
        }
        String::new()
    }
}


#[derive(Default)]
pub struct PlayerAngProvider {
}

impl PlayerAngProvider {
}

impl StatsProvider for PlayerAngProvider {
    fn get_label(&self) -> String {
        "Player Look".to_string()
    }

    fn priority(&self) -> i32 { -4 }

    fn format_value(&self, world: &mut World) -> String {
        let mut look_q = world.query_filtered::<&PlayerLook, With<Player>>();
        for look in look_q.iter(world) {
            let (y, x, _) = look.rotation.to_euler(EulerRot::default());
            return format!("{:.1?} / {:.1?}", y.to_degrees(), x.to_degrees());
        }
        String::new()
    }
}

fn add_default_providers(mut regy: ResMut<StatsRegistry>) {
    regy.add_provider(Box::new(FpsProvider));
    regy.add_provider(Box::new(FpsMaxProvider));
    regy.add_provider(Box::new(EntCountProvider));
    regy.add_provider(Box::new(ContactCountProvider));
    regy.add_provider(Box::new(CpuUsageProvider::default()));
    regy.add_provider(Box::new(MemoryUsageProvider::default()));
    regy.add_provider(Box::new(PlayerPosProvider::default()));
    regy.add_provider(Box::new(PlayerAngProvider::default()));
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

#[allow(clippy::too_many_arguments)]
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

            let plain_color = Color::Srgba(bevy::color::palettes::tailwind::GRAY_50);
            let important_color = Color::Srgba(bevy::color::palettes::tailwind::RED_500);

            let mut result  = Vec::with_capacity(stats_registry.len());
            let font = TextFont {
                font_size: style.font_size,
                font: style.font.clone(),
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
                        .for_each(|provider| result.push(c.spawn((
                            Node::default(),
                            font.clone(),
                            Text::default(),
                            TextColor(if provider.is_important() { important_color } else { plain_color }),
                        )).id()
                    ));
                });
            });

            result
        }).clone();

        *refresh_timer += time.delta_secs();
        if *refresh_timer > 0.05 {
            *refresh_timer = 0.;

            let _ = world.resource_scope::<StatsRegistry, Result>(|world, stats_registry| {
                // let Some(stats_registry) = world.get_resource::<StatsRegistry>() else { return };
                let values = stats_registry.providers().iter().map(|prov| prov.format_value(world)).collect::<Vec<_>>();

                let mut text = world.query::<&mut Text>();
                for (index, value) in values.into_iter().enumerate() {
                    if let Ok(mut text) = text.get_mut(world, text_ents[index]) {
                        text.0.clear();
                        text.0 = value;
                    }
                }
                Ok(())
            });
        }

    }
    queue.apply(world);
}
