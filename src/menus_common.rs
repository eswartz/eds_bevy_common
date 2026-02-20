//! Stock widgertry for handling menus.
use std::collections::HashMap;
use std::hash::BuildHasher as _;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use bevy::color::palettes::tailwind;
use bevy::ecs::system::SystemId;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::InputFocus;
use bevy::input_focus::InputFocusVisible;
use bevy::input_focus::tab_navigation::NavAction;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigation;
use bevy::input_focus::tab_navigation::TabNavigationError;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::camera::visibility::RenderLayers;
use bevy::text::LineHeight;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;
use rustc_hash::FxBuildHasher;

use crate::RENDER_LAYER_UI;
use crate::is_in_menu;

use super::states_sets::OverlayState;
use super::states_sets::ProgramState;

const MARGIN: Val = Val::Px(16.);
const SHADOW_OFFSET: f32 = 4.0;

const MENU_ITEM_FONT_SIZE: f32 = 40.0;

const TEXT_SHADOW_COLOR: Color = Color::hsva(0.0, 0.0, 0.25, 1.0);
const NORMAL_BUTTON: Color = Color::hsva(0.0, 0.0, 0.75, 1.0);
const HOVERED_BUTTON: Color = Color::hsva(0.0, 0.0, 1.0, 1.0);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

pub struct MenuCommonPlugin;
impl Plugin for MenuCommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputDispatchPlugin)
            .add_plugins(TabNavigationPlugin)
            .insert_resource(DraggingMenuItem(None))
            .insert_resource(MenuItemSelectionHistory::default())
            .insert_resource(PreviousMenuStack::default())
            .add_message::<MenuActionMessage>()
            .add_systems(
                PreUpdate,
                // Common handling for any menu.
                (
                    on_added_menu_item,
                    handle_menu_keys,
                    handle_menu_action,
                    handle_menu_item_decoration,
                    handle_menu_keys_navigation,
                    handle_menu_mouse_drag,
                    handle_menu_mouse_click,
                    handle_menu_enums_init,
                    handle_menu_enums_actions,
                    handle_menu_enums_refresh,
                    handle_menu_toggle_init,
                    handle_menu_toggle_actions,
                    handle_menu_toggle_refresh,
                    handle_menu_slider_init,
                    handle_menu_slider_actions,
                    handle_menu_slider_refresh,
                )
                .run_if(is_in_menu)
            )
            .add_systems(
                Update,
                (
                    handle_menu_back,
                    handle_menu_into,
                )
            )

        ;
    }
}

fn handle_menu_back(mut commands: Commands,
    go_back_in_menu_request: Option<Res<GoBackInMenuRequest>>,
    overlay_state: Res<State<OverlayState>>,
    mut prev_menu: ResMut<PreviousMenuStack>,
) {
    // Inner filtering for clarity.
    if go_back_in_menu_request.is_none() {
        return
    }

    // Ok, pop something.
    if let Some(prev) = prev_menu.0.pop() {
        commands.set_state(prev);
    } else if *overlay_state.get() != OverlayState::MainMenu {
        // Done, no more history.
        commands.set_state(OverlayState::Hidden);
    }

    // Handled, whether or not it did anything.
    commands.remove_resource::<GoBackInMenuRequest>();
}

fn handle_menu_into(mut commands: Commands,
    go_into_in_menu_request: Option<Res<GoIntoMenuRequest>>,
    overlay_state: Res<State<OverlayState>>,
    mut prev_menu: ResMut<PreviousMenuStack>,
) {
    // Inner filtering for clarity.
    let Some(request) = go_into_in_menu_request else {
        return;
    };

    let current = *overlay_state.get();
    let to_enter: OverlayState = request.0;
    if current == to_enter {
        return;
    }

    // Shall we remember this?
    let exiting = current;
    if to_enter.is_menu() {
        if let Some(index) = prev_menu.0.iter().position(|x| *x == exiting) {
            // We hit a loop; clip at the earliest instance.
            prev_menu.0.truncate(index);
        }
        prev_menu.0.push(exiting);

        // Do it.
        commands.set_state(to_enter);
    } else {
        log::warn!("not a menu: {to_enter:?}");
    }

    // Handled, whether or not it did anything.
    commands.remove_resource::<GoIntoMenuRequest>();
}

pub struct MenuItemBuilder<'w, 's> {
    page: Entity,
    overlay: OverlayState,
    font: Handle<Font>,
    font_scale: f32,
    commands: Commands<'w, 's>,
    item_index: i32,
    previous_first_ent_label: Option<(Entity, String)>,
    first_ent_label: Option<(Entity, String)>,
}

#[allow(unused)]
impl<'w, 's> MenuItemBuilder<'w, 's> {
    pub fn new(
        mut commands: Commands<'w, 's>,
        overlay: OverlayState,
        program: ProgramState,
        font: Handle<Font>,
        font_scale: f32,
        history: &MenuItemSelectionHistory,
    ) -> Self {
        let page = commands
            .spawn((
                DespawnOnExit(overlay),
                create_menu_page_node(),
                TabGroup::new(FxBuildHasher.hash_one(overlay) as _),
            ))
            .with_child(
                Node {
                    margin: UiRect {
                        top: Val::Px(if program == ProgramState::LaunchMenu { 128.0 } else { 0.0 }),
                        ..default()
                    },
                    ..default()
                }
            )
            .id();

        Self {
            overlay,
            font,
            font_scale,
            page,
            commands,
            item_index: 0,
            previous_first_ent_label: history.0.get(&overlay).cloned(),
            first_ent_label: None,
        }
    }

    pub fn add_item(
        &mut self,
        text: impl Into<String>,
        other_menu_items: impl Bundle,
        handler: impl MenuItemHandler + 'static,
    ) -> &mut Self {
        let text: String = text.into();
        let tab_index = TabIndex(self.item_index);
        let mut ent_commands = self.commands.entity(self.page);
        let mut ent_id = Entity::PLACEHOLDER;
        ent_commands.with_children(|builder| {
            ent_id = spawn_menu_button_bundle(
                builder,
                text.clone(),
                self.font.clone(),
                self.font_scale,
                tab_index,
                Arc::new(Mutex::new(handler)),
            );
            builder
                .commands()
                .entity(ent_id)
                .insert(Name::new(text.clone()))
                .insert(other_menu_items);
        });

        self.item_index += 1;

        // Track the user first item so we can auto-focus it.
        if self.first_ent_label.is_none()
            || self
                .previous_first_ent_label
                .as_ref()
                .is_some_and(|(_, label)| *label == text)
        {
            self.first_ent_label = Some((ent_id, text.to_string()));
        }
        self
    }

    pub fn add_label(
        &mut self,
        text: impl Into<String>,
        font_scale_scale: f32,
        other_menu_items: impl Bundle,
    ) -> &mut Self {
        let text: String = text.into();
        let mut ent_commands = self.commands.entity(self.page);
        let mut ent_id = Entity::PLACEHOLDER;
        ent_commands.with_children(|builder| {
            ent_id = spawn_menu_label_bundle(
                builder,
                text.clone(),
                self.font.clone(),
                self.font_scale * font_scale_scale,
            );
            builder
                .commands()
                .entity(ent_id)
                .insert(Name::new(text.clone()))
                .insert(other_menu_items);
        });
        self
    }

    /// Apply the builder to the world.
    ///
    /// Most of the commands to create the menu are
    /// already created on-the-fly during the non-consuming
    /// methods of Self.
    ///
    /// This last step forces focus and styling if
    /// `first_ent_label` is Some((Entity, String))
    /// (the relevant menu item, with a menu item
    /// component from this module).
    /// It maintains a path history regarding the "last" spot
    /// navigated in the menu,
    /// so forward and backward navigation are symmetric when
    /// reentering the menu structure.
    pub fn finish(&mut self, history: &mut MenuItemSelectionHistory) {
        if let Some((first_ent, label)) = self.first_ent_label.take() {
            history
                .0
                .insert(self.overlay, (first_ent, label).clone());
            self.commands.insert_resource(InputFocus(Some(first_ent)));
            self.commands.insert_resource(RefreshMenu);
            self.commands.entity(first_ent).insert(Interaction::Hovered);
            self.commands.write_message(MenuActionMessage::Navigate(first_ent));

            // self.prev.0.push(self.overlay);
            // dbg!(&self.prev.0);
            // self.commands.insert_resource(self.prev.clone());
        }
    }
}

// #[derive(Resource, Default, Debug)]
// pub struct PauseStateBeforeMenu(bool);

/// Temporary resource, requests redraw of the current menu.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshMenu;

/// Previous menu items.
#[derive(Resource, Debug, Default)]
pub struct MenuItemSelectionHistory(HashMap<OverlayState, (Entity, String)>);

/// Menu stack, where new states are added on entry, and popped on
/// SimpleMenuAction::Back operations.
#[derive(Resource, Debug, Default, Clone)]
pub struct PreviousMenuStack(pub Vec<OverlayState>);

/// A client defines this resource to request going "back" from a menu state
/// (whether programmatic or user-event driven).
/// When handled, it is removed and OverlayState is changed if possible.
#[derive(Resource, Debug, Default, Clone)]
pub struct GoBackInMenuRequest;

/// A client defines this resource to request going into a specific menu.
/// This specific form, vs. just setting the OverlayState directly,
/// implies keeping history via `PreviousMenuStack`.
#[derive(Resource, Debug, Default, Clone)]
pub struct GoIntoMenuRequest(pub OverlayState);

/////

/// Event firing a menu action.
#[derive(Message, Debug, Clone)]
pub enum MenuActionMessage {
    /// Navigate(d) to new menu.
    Navigate(Entity),
    /// Activate the given item (menu/toggle/slider).
    Activate(Entity),
    /// Reset the menu item to default.
    Reset(Entity),
    /// Select the next item (menu/enum/toggle/slider).
    Next(Entity),
    /// Select the previous item (menu/enum/toggle/slider).
    Previous(Entity),
    /// Slide up or down by the given value.
    /// The value is a measure of the drag distance divided by the window width,
    /// thus typically is in the range -1...1 (at the extremes).
    Slide(Entity, f32),
}

impl MenuActionMessage {
    pub fn entity(&self) -> Entity {
        match self {
            MenuActionMessage::Navigate(entity)
            | MenuActionMessage::Activate(entity)
            | MenuActionMessage::Reset(entity)
            | MenuActionMessage::Next(entity)
            | MenuActionMessage::Previous(entity)
            | MenuActionMessage::Slide(entity, _) => *entity,
        }
    }
}

/// Component marking a menu item. The handler responds to [MenuActionMessage]s.
#[derive(Component)]
struct MenuItem(Arc<Mutex<dyn MenuItemHandler>>);

/// Original text and scale for dynamic menu items.
#[derive(Component, Debug, Clone)]
pub struct MenuBaseText(pub String, pub f32);

/// Marker for menu items that act as enumerants.
#[derive(Component, Clone)]
pub struct MenuEnum {
    /// Current model value (index within some list), cached from init.
    pub current: Option<usize>,
    /// System that fetches the value from the model and then sets it in Self::current.
    pub get: SystemId<In<Entity>, ()>,
    /// System that applies the given value to the model.
    pub set: SystemId<In<usize>, ()>,
    /// How many items are there? Given the return, [0..count()) are valid.
    pub count: Arc<dyn Fn() -> usize + 'static + Send + Sync>,
    /// Get the string for the value.
    pub display: Arc<dyn Fn(usize) -> String + 'static + Send + Sync>,
}

impl MenuEnum {
    /// `get`: System that fetches the model value then sets it in Self::current.
    /// `set`: System that applies the given value to the model.
    /// `count`: How many items are there? Given the return, [0..count()) are valid.
    /// `display`: Get the display value of the selected index.
    pub fn new(
        get: SystemId<In<Entity>>,
        set: SystemId<In<usize>, ()>,
        count: impl Fn() -> usize + 'static + Send + Sync,
        display: impl Fn(usize) -> String + 'static + Send + Sync,
    ) -> Self {
        Self {
            current: None,
            get,
            set,
            count: Arc::new(count),
            display: Arc::new(display),
        }
    }
}

/// Marker for menu items that act as toggles.
#[derive(Component, Debug, Clone)]
pub struct MenuToggle {
    /// Current model value, cached from init.
    pub current: Option<bool>,
    /// System that fetches the value from the model and then sets it in Self::current.
    pub get: SystemId<In<Entity>, ()>,
    /// System that applies the given value to the model.
    pub set: SystemId<In<bool>, ()>,
}

impl MenuToggle {
    /// `get`: System that fetches the model value then sets it in Self::current.
    /// `set`: System that applies the given value to the model.
    pub fn new(get: SystemId<In<Entity>>, set: SystemId<In<bool>, ()>) -> Self {
        Self {
            current: None,
            get,
            set,
        }
    }
}

/// Marker for menu items that act as sliders.
#[derive(Component, Clone)]
pub struct MenuSlider {
    /// Current model value, cached from init.
    pub current: Option<f32>,
    /// System that fetches the value from the model and then sets it in MenuSlider::current.
    pub get: SystemId<In<Entity>, ()>,
    /// System that applies the given value to the model.
    pub set: SystemId<In<f32>, ()>,
    /// Get the default model value.
    pub default_fn: Arc<dyn Fn() -> Option<f32> + 'static + Send + Sync>,
    /// Convert the model value to UI.
    pub to_ui_fn: Arc<dyn Fn(f32) -> f32 + 'static + Send + Sync>,
    /// Convert the UI value to model.
    pub from_ui_fn: Arc<dyn Fn(f32) -> f32 + 'static + Send + Sync>,
    /// The UI range for the value. User values will be clamped to this.
    pub ui_range: RangeInclusive<f32>,
    /// The UI step basis. This affects how the value is minimally incremented.
    pub ui_step_base: f32,
}


/// When this resource is Some, we're dragging this.
#[derive(Resource, Debug)]
pub struct DraggingMenuItem(pub Option<Entity>);

impl MenuSlider {
    /// `get`: System that fetches the model value then sets it in MenuSlider::current.
    /// `set`: System that applies the given value to the model.
    /// `default`: Function providing the default model value.
    /// `to_ui`: Function mapping the model value to UI.
    /// `from_ui`: Function mapping the UI value to model.
    /// `ui_range`: Allowed values in UI slider.
    /// `ui_step_base`: Basic stepping for keyboard or mouse movement.
    pub fn new(
        get: SystemId<In<Entity>>,
        set: SystemId<In<f32>, ()>,
        default: impl Fn() -> Option<f32> + 'static + Send + Sync,
        to_ui: impl Fn(f32) -> f32 + 'static + Send + Sync,
        from_ui: impl Fn(f32) -> f32 + 'static + Send + Sync,
        ui_range: RangeInclusive<f32>,
        ui_step_base: f32,
    ) -> Self {
        Self {
            current: None,
            get,
            set,
            default_fn: Arc::new(default),
            to_ui_fn: Arc::new(to_ui),
            from_ui_fn: Arc::new(from_ui),
            ui_range,
            ui_step_base,
        }
    }

    #[allow(unused)]
    pub fn range(&self) -> f32 {
        *self.ui_range.end() - *self.ui_range.start()
    }

    pub fn add_and_clamp(&self, delta: f32, ui_orig: f32) -> f32 {
        let delta = if delta.abs() < self.ui_step_base { self.ui_step_base.copysign(delta) } else { (delta * self.ui_step_base).round() / self.ui_step_base };
        (ui_orig + delta).clamp(*self.ui_range.start(), *self.ui_range.end())
    }
}

/// Implementation for a specific menu item (or group).
pub trait MenuItemHandler: Send + Sync {
    /// Handle the given event on the event's menu entity.
    fn handle(&mut self, world: &mut World, event: &MenuActionMessage) {
        let _ = world;
        let _ = event;
    }
}

fn create_menu_page_node() -> (Node, BackgroundColor, RenderLayers) {
    (
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(MARGIN),
            ..default()
        },
        BackgroundColor(tailwind::BLUE_900.with_alpha(0.25).into()),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),
    )
}

fn spawn_menu_button_bundle(
    builder: &mut ChildSpawnerCommands,
    text: String,
    font: Handle<Font>,
    font_size_scale: f32,
    tab_index: TabIndex,
    handler: Arc<Mutex<dyn MenuItemHandler>>,
) -> Entity {
    let ent_id = spawn_menu_row_bundle(builder, font, font_size_scale);
    builder.commands().entity(ent_id).insert((
        Button,
        Text::new(text.clone()),
        MenuBaseText(text, font_size_scale),
        tab_index,
        MenuItem(handler),
        RelativeCursorPosition::default(),
    ));
    ent_id
}

fn spawn_menu_label_bundle(
    builder: &mut ChildSpawnerCommands,
    text: String,
    font: Handle<Font>,
    font_size_scale: f32,
) -> Entity {
    let ent_id = spawn_menu_row_bundle(builder, font, font_size_scale);
    builder.commands().entity(ent_id).insert((
        Text::new(text.clone()),
        MenuBaseText(text, font_size_scale),
    ));
    ent_id
}

/// Add a menu item row to a given menu page.
pub fn spawn_menu_row_bundle(
    builder: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    font_size_scale: f32,
) -> Entity {
    builder
        .spawn((
            Node {
                padding: UiRect::axes(MARGIN, MARGIN),
                align_self: AlignSelf::Center,
                height: Val::Px(MENU_ITEM_FONT_SIZE * font_size_scale * 2.0),
                ..default()
            },
            TextFont {
                font,
                font_size: MENU_ITEM_FONT_SIZE * font_size_scale,
                ..default()
            },
            LineHeight::RelativeToFont(font_size_scale),
            TextColor::WHITE,
            TextShadow {
                color: TEXT_SHADOW_COLOR,
                offset: Vec2::splat(SHADOW_OFFSET * font_size_scale),
            },
        ))
        .id()
}

fn on_added_menu_item(
    mut commands: Commands,
    item_q: Query<(Entity, Has<MenuToggle>, Has<MenuEnum>, Has<MenuSlider>), Added<MenuItem>>,
) {
    for (ent, toggle, enm, slider) in item_q.iter() {
        let mut ent_commands = commands.entity(ent);
        ent_commands
            .observe(
                move |mut trigger: On<Pointer<Press>>,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      slider_q: Query<&MenuSlider>,
                      mut dragging: ResMut<DraggingMenuItem>| {
                    if focus.0.is_none() || slider_q.get(trigger.event().entity).iter().next().is_none() {
                        focus.set(trigger.event().entity);
                        visible.0 = true;
                        dragging.0 = None;
                        trigger.propagate(false);
                    }
                },
            )
            .observe(
                move |mut trigger: On<Pointer<Click>>,
                      mut commands: Commands,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      slider_q: Query<&MenuSlider>,
                      dragging: Res<DraggingMenuItem>| {
                    let was_focused = focus.0 == Some(trigger.event().entity);
                    focus.set(trigger.event().entity);
                    visible.0 = true;
                    if was_focused
                        && (dragging.0.is_none()
                            && (slider_q.get(trigger.event().entity).iter().next().is_none()
                                || trigger.event().duration < Duration::from_millis(100)
                            ))
                    {
                        commands.write_message(MenuActionMessage::Activate(trigger.event().entity));
                        trigger.propagate(false);
                    }
                },
            );

        if toggle {
            ent_commands.observe(
                move |trigger: On<Pointer<Click>>,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      dragging: Option<Res<DraggingMenuItem>>,
                      mut writer: MessageWriter<MenuActionMessage>| {
                    focus.set(trigger.event().entity);
                    visible.0 = true;
                    if dragging.is_none() {
                        writer.write(MenuActionMessage::Activate(trigger.event().entity));
                    }
                },
            );
        }

        if enm {
            ent_commands.observe(
                move |trigger: On<Pointer<Click>>,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      dragging: Option<Res<DraggingMenuItem>>,
                      mut writer: MessageWriter<MenuActionMessage>| {
                    focus.set(trigger.event().entity);
                    visible.0 = true;
                    if dragging.is_none() {
                        writer.write(MenuActionMessage::Next(trigger.event().entity));
                    }
                },
            );
        }

        if slider || enm {
            ent_commands
                .observe(
                    move |trigger: On<Pointer<DragStart>>,
                          mut focus: ResMut<InputFocus>,
                          mut visible: ResMut<InputFocusVisible>,
                          mut dragging: ResMut<DraggingMenuItem>| {
                        focus.set(trigger.event().entity);
                        visible.0 = true;
                        dragging.0 = Some(trigger.event().entity);
                    },
                )
                .observe(
                    move |trigger: On<Pointer<Drag>>,
                          mut dragging: ResMut<DraggingMenuItem>,
                          window: Single<&Window, With<PrimaryWindow>>,
                          mut writer: MessageWriter<MenuActionMessage>| {
                        writer.write(MenuActionMessage::Slide(
                            trigger.event().entity,
                            trigger.event().distance.x / window.width(),
                        ));
                        dragging.0 = Some(trigger.event().entity);
                    },
                )
                .observe(
                    move |mut trigger: On<Pointer<DragEnd>>,
                          mut dragging: ResMut<DraggingMenuItem>| {
                        dragging.0 = None;
                        trigger.propagate(false);
                    },
                );
        }
    }
}

fn handle_menu_keys(
    mut commands: Commands,
    mut reader: MessageReader<KeyboardInput>,
    focus: Res<InputFocus>,
    toggle_q: Query<&MenuToggle>,
    slider_q: Query<&MenuSlider>,
    enum_q: Query<&MenuEnum>,
    mut writer: MessageWriter<MenuActionMessage>,
) {
    let Some(entity) = focus.0 else { return };

    for key_event in reader.read() {
        if key_event.state == ButtonState::Pressed {
            if key_event.key_code == KeyCode::Escape {
                commands.insert_resource(GoBackInMenuRequest);
            }
            else if key_event.key_code == KeyCode::ArrowLeft {
                if slider_q.contains(entity) {
                    writer.write(MenuActionMessage::Slide(
                        entity,
                        -1.0,
                    ));
                } else if toggle_q.contains(entity) || enum_q.contains(entity) {
                    writer.write(MenuActionMessage::Previous(entity));
                }
            } else if key_event.key_code == KeyCode::ArrowRight {
                if slider_q.contains(entity) {
                    writer.write(MenuActionMessage::Slide(
                        entity,
                        1.0,
                    ));
                } else if toggle_q.contains(entity) || enum_q.contains(entity) {
                    writer.write(MenuActionMessage::Next(entity));
                }
            }
        }
    }
}

fn handle_menu_keys_navigation(
    mut commands: Commands,
    nav: TabNavigation,
    mut key_reader: MessageReader<KeyboardInput>,
    menu_item_q: Query<&MenuItem>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
) {
    // Tab navigation.
    for key_event in key_reader.read() {
        if key_event.state == ButtonState::Pressed && !key_event.repeat {
            // Activate menu item?
            if key_event.key_code == KeyCode::Enter || key_event.key_code == KeyCode::Space {
                if let Some(ent) = &focus.0
                    && menu_item_q.contains(*ent)
                {
                    commands.write_message(MenuActionMessage::Activate(*ent));
                } else {
                    warn!("no MenuItem");
                }
                continue;
            }

            // Reset to default?
            if key_event.key_code == KeyCode::Backspace {
                if let Some(ent) = &focus.0
                    && menu_item_q.contains(*ent)
                {
                    commands.write_message(MenuActionMessage::Reset(*ent));
                } else {
                    warn!("no MenuItem");
                }
                continue;
            }

            // Move in menu?
            let nav_dir = match key_event.key_code {
                KeyCode::ArrowDown => NavAction::Next,
                KeyCode::ArrowUp => NavAction::Previous,
                _ => continue,
            };

            let maybe_next = nav.navigate(&focus, nav_dir);

            match maybe_next {
                Ok(next) => {
                    focus.set(next);
                    visible.0 = true;
                    commands.write_message(MenuActionMessage::Navigate(next));
                }
                Err(e) => {
                    // This failure mode is recoverable, but still indicates a problem.
                    // warn!("Tab navigation error: {}", e);
                    if let TabNavigationError::NoTabGroupForCurrentFocus { new_focus, .. } = e {
                        focus.set(new_focus);
                        visible.0 = true;
                        commands.write_message(MenuActionMessage::Navigate(new_focus));
                    }
                }
            }
        }
    }
}

#[derive(Resource)]
struct MousePressedDuration {
    ent: Entity,
    time: Duration,
}

fn handle_menu_mouse_drag(
    mut commands: Commands,
    mut reader: MessageReader<Pointer<Drag>>,
    mut dragging: ResMut<DraggingMenuItem>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut writer: MessageWriter<MenuActionMessage>,
    focus: Res<InputFocus>,
) {
    if let Some(focus) = focus.0.as_ref() {
        for event in reader.read() {
            writer.write(MenuActionMessage::Slide(
                *focus,
                event.distance.x / window.width(),
            ));
            dragging.0 = Some(*focus);
            commands.remove_resource::<MousePressedDuration>();
        }
    }
}

fn handle_menu_mouse_click(
    mut commands: Commands,
    int_q: Query<(Entity, &Interaction), (Changed<Interaction>, With<MenuItem>)>,
    slider_q: Query<&MenuSlider>,
    time: Res<Time>,
    pressed: Option<Res<MousePressedDuration>>,
    mut focus: ResMut<InputFocus>,
) {
    for (ent, int) in int_q.iter() {
        if *int == Interaction::Pressed {
            // Queue to activate menu item on release.
            commands.insert_resource(MousePressedDuration{ ent, time: time.elapsed() });
            focus.set(ent);
        }
        else if let Some(ref pressed) = pressed
        && pressed.ent == ent
        && if slider_q.contains(ent) { time.elapsed().saturating_sub(pressed.time) >= Duration::from_millis(250) } else { true }
            && *int == Interaction::Hovered {
                commands.write_message(MenuActionMessage::Activate(ent));
                commands.remove_resource::<MousePressedDuration>();
            }
    }
}

fn handle_menu_item_decoration(
    mut interaction_query: Query<
        (
            Entity,
            &Interaction,
            &MenuBaseText,
            &mut TextFont,
            &mut TextColor,
        ),
        With<Button>,
    >,
    focus: Res<InputFocus>,
) {
    for (ent, interaction, MenuBaseText(_, scale), mut text, mut color) in &mut interaction_query {
        let (font_size, item_color) = match (*interaction, focus.0 == Some(ent)) {
            (_, true) => (MENU_ITEM_FONT_SIZE * scale * 1.1, PRESSED_BUTTON.into()),
            (Interaction::Pressed, _) => (MENU_ITEM_FONT_SIZE * scale * 1.1, PRESSED_BUTTON.into()),
            (Interaction::Hovered, _) => (MENU_ITEM_FONT_SIZE * scale * 1.1, HOVERED_BUTTON.into()),
            (Interaction::None, _) => (MENU_ITEM_FONT_SIZE * scale, NORMAL_BUTTON.into()),
        };
        if text.font_size != font_size || *color != item_color {
            text.font_size = font_size;
            *color = item_color;
        }
    }
}

fn handle_menu_action(world: &mut World) {
    let mut reader = IntoSystem::into_system(
        |mut events: MessageReader<MenuActionMessage>| -> Vec<MenuActionMessage> {
            events.read().cloned().collect::<Vec<_>>()
        },
    );
    reader.initialize(world);
    let Ok(events) = reader.run((), world) else {
        log::error!("failed to fetch menu action events");
        return
    };

    let overlay_state = *world
        .get_resource::<State<OverlayState>>()
        .expect("expected OverlayState")
        .get();

    for event in events {
        let menu_item = event.entity();
        let handler = {
            let mut menu_item_q = world.query::<(
                &MenuItem,
                &MenuBaseText,
                Option<&MenuToggle>,
                Option<&MenuSlider>,
            )>();

            let Ok((item, text, _, _)) = menu_item_q.get(world, menu_item) else {
                return;
            };

            let handler = item.0.clone();
            let text = text.0.clone();

            if let Some(mut history) = world.get_resource_mut::<MenuItemSelectionHistory>() {
                history.0.insert(overlay_state, (menu_item, text));
            }

            handler
        };

        handler.lock().unwrap().handle(world, &event);

        world.insert_resource(RefreshMenu);
    }
}

fn handle_menu_slider_init(
    mut commands: Commands,
    mut slider_q: Query<(Entity, &mut MenuSlider), Added<MenuSlider>>,
) {
    for (entity, slider) in slider_q.iter_mut() {
        commands.run_system_with(slider.get, entity);
    }
}

/// Get the Unicode icon for filled or unfilled ballot box (i.e. checkbox).
fn check(b: bool) -> &'static str {
    if b { "\u{2611}" } else { "\u{2610}" }
}

fn handle_menu_slider_refresh(
    mut slider_q: Query<
        (&mut Text, &MenuBaseText, &MenuSlider, Option<&MenuToggle>),
        Or<(Changed<MenuSlider>, Changed<MenuToggle>)>
    >,
) {
    for (mut text, base_text, slider, toggle) in slider_q.iter_mut() {
        let Some(value) = slider.current.as_ref() else {
            continue;
        };
        let mut msg = format!("{} ({:.1})", &base_text.0, (slider.to_ui_fn)(*value));
        // Allow slider with toggle.
        if let Some(toggle) = toggle {
            let Some(value) = toggle.current.as_ref() else {
                continue;
            };
            msg = format!("{} {}", check(*value), msg);
        }
        text.0 = msg;
    }
}

fn handle_menu_slider_actions(
    mut commands: Commands,
    mut reader: MessageReader<MenuActionMessage>,
    mut slider_q: Query<&mut MenuSlider>,
) {
    for event in reader.read() {
        let entity = event.entity();

        let Ok(mut slider) = slider_q.get_mut(entity) else {
            continue;
        };

        match event {
            MenuActionMessage::Navigate(_) |
            MenuActionMessage::Activate(_) => (),
            MenuActionMessage::Reset(_) => {
                if let Some(default) = (slider.default_fn)() {
                    // Reset.
                    commands.run_system_with(slider.set, default);
                    // Force refresh.
                    commands.run_system_with(slider.get, entity);
                }
            }
            MenuActionMessage::Slide(_, delta) => {
                // Adjust value in UI space.
                let Some(current) = slider.current.as_mut() else {
                    error!("slider not initialized yet");
                    continue;
                };
                let current = *current;

                let orig = (slider.to_ui_fn)(current);
                let new = (slider.from_ui_fn)(slider.add_and_clamp(*delta, orig));

                commands.run_system_with(slider.set, new);
                // Refresh from internal value.
                commands.run_system_with(slider.get, entity);
            }
            MenuActionMessage::Next(_) | MenuActionMessage::Previous(_) => (),
        }
    }
}

fn handle_menu_toggle_init(
    mut commands: Commands,
    mut toggle_q: Query<(Entity, &mut MenuToggle), Added<MenuToggle>>,
) {
    for (entity, toggle) in toggle_q.iter_mut() {
        commands.run_system_with(toggle.get, entity);
    }
}

fn handle_menu_toggle_refresh(
    mut toggle_q: Query<
        (&mut Text, &MenuBaseText, &MenuToggle),
        (Changed<MenuToggle>, Without<MenuSlider>),
    >,
) {
    for (mut text, base_text, toggle) in toggle_q.iter_mut() {
        let Some(value) = toggle.current.as_ref() else {
            continue;
        };
        text.0 = format!("{} {}", check(*value), base_text.0);
    }
}

fn handle_menu_toggle_actions(
    mut commands: Commands,
    mut reader: MessageReader<MenuActionMessage>,
    mut toggle_q: Query<&mut MenuToggle>,
) {
    for event in reader.read() {
        let entity = event.entity();

        let Ok(mut toggle) = toggle_q.get_mut(entity) else {
            continue;
        };

        if !matches!(event, MenuActionMessage::Activate(_))
        {
            continue
        }
        let Some(current) = toggle.current.as_mut() else {
            error!("toggle not initialized yet");
            continue;
        };

        let new = !*current;
        commands.run_system_with(toggle.set, new);
        // Refresh from internal value.
        commands.run_system_with(toggle.get, entity);
    }
}

fn handle_menu_enums_init(
    mut commands: Commands,
    mut enums_q: Query<(Entity, &mut MenuEnum), Added<MenuEnum>>,
) {
    for (entity, enums) in enums_q.iter_mut() {
        commands.run_system_with(enums.get, entity);
    }
}

fn handle_menu_enums_refresh(
    mut enums_q: Query<
        (&mut Text, &MenuBaseText, &MenuEnum),
        Changed<MenuEnum>
    >,
) {
    for (mut text, base_text, enums) in enums_q.iter_mut() {
        let Some(value) = enums.current.as_ref() else {
            continue;
        };
        let disp = (enums.display)(*value);
        text.0 = format!("{}: {}", &base_text.0, disp);
    }
}

fn handle_menu_enums_actions(
    mut commands: Commands,
    mut reader: MessageReader<MenuActionMessage>,
    mut enums_q: Query<&mut MenuEnum>,
    mut slider_time: Local<Duration>,
    time: Res<Time>,
) {
    for event in reader.read() {
        let entity = event.entity();

        let Ok(mut enums) = enums_q.get_mut(entity) else {
            continue;
        };

        let dir = match event {
            MenuActionMessage::Slide(_, distance) => {
                if distance.abs() >= 0.1 {
                    *slider_time = slider_time.saturating_sub(time.delta());
                    if slider_time.is_zero() {
                        *slider_time = Duration::from_secs_f32(1.0 / 2.0);
                        distance.signum() as isize
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            MenuActionMessage::Next(_) => 1,
            MenuActionMessage::Previous(_) => -1,
            _ => 0,
        };
        if dir == 0 {
            continue
        }

        // Adjust value in UI space.
        let Some(current) = enums.current.as_mut() else {
            error!("enum not initialized yet");
            continue;
        };
        let current = *current;
        let count = (enums.count)();

        let new = ((current as isize) + dir).rem_euclid(count as isize) as usize;

        commands.run_system_with(enums.set, new);
        // Refresh from internal value.
        commands.run_system_with(enums.get, entity);
    }
}
