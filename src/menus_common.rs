//! Stock widgetry for handling menus.
//!
//! Client code typically sets up like this:
//!
//! ```
//! pub struct MyMenuPlugin;
//! impl Plugin for MyMenuPlugin {
//!     fn build(&self, app: &mut App) {
//!         app
//!             .add_plugins(MenuCommonPlugin)
//!             .add_systems(OnEnter(OverlayState::MainMenu), on_enter_main_menu)
//!             .add_systems(OnEnter(OverlayState::EscapeMenu), on_enter_escape_menu)
//!             .add_systems(OnExit(OverlayState::EscapeMenu), on_exit_escape_menu)
//!             .add_systems(OnEnter(OverlayState::GameMenu), on_enter_game_menu)
//!             .add_systems(OnEnter(OverlayState::OptionsMenu), on_enter_options_menu)
//!             .add_systems(OnEnter(OverlayState::AudioMenu), on_enter_audio_menu)
//!             .add_systems(OnEnter(OverlayState::VideoMenu), on_enter_video_menu)
//!             .add_systems(OnEnter(OverlayState::ControlsMenu), on_enter_controls_menu);
//!     }
//! }
//! ```
//! where the `on_enter...` functions create menus using [MenuItemBuilder] with
//! appropriate items, then activates a menu by setting the [OverlayState] appropriately.
//!
//! [`ProgramState::LaunchMenu`] will trigger [OverlayState::MainMenu] via handlers in
//! [crate::app::AppPlugin].
//!
//! While a menu is up [OverlayState::is_menu], user keyboard/mouse/controller
//! actions will be interpreted. Client code can insert [GoIntoMenuRequest]
//! to stack a submenu and [GoBackInMenuRequest] to pop an entry.
//!
//! [crate::actions_common::handle_escape] handles entering [OverlayState::EscapeMenu]
//! during gameplay and also unstacking menus, but never goes past [OverlayState::MainMenu].
//!
use bevy::input_focus::AutoFocus;
use bevy::input_focus::FocusCause;
use bevy::picking::hover::Hovered;
use bevy::platform::collections::HashMap;
use bevy::ui::Pressed;
use bevy::ui_widgets::Button;
use std::hash::BuildHasher as _;
use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::tailwind;
use bevy::ecs::system::SystemId;
use bevy::input_focus::InputFocus;
use bevy::input_focus::InputFocusVisible;
use bevy::input_focus::tab_navigation::*;
use bevy::prelude::*;
use bevy::text::LineHeight;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;
#[cfg(feature = "input_bei")]
use bevy_enhanced_input::prelude::*;
use rustc_hash::FxBuildHasher;

use crate::gui::UiFont;
use crate::prelude::RENDER_LAYER_UI;
#[cfg(feature = "input_bei")]
use crate::prelude::actions;
use crate::prelude::is_in_menu;

use super::states_sets::OverlayState;
use super::states_sets::ProgramState;

const MARGIN: Val = Val::Px(16.);
const SHADOW_OFFSET: f32 = 4.0;

const MENU_ITEM_FONT_SIZE: Val = Val::Px(40.0);

/// Style settings for the menu.
#[derive(Debug, Clone, PartialEq, Resource, Reflect)]
#[type_path = "game"]
pub struct MenuStyle {
    /// Color for the background in menus.
    /// This is applied by [MenuItemBuilder::page].
    pub page_color: Option<Color>,
    pub page_image: Option<Handle<Image>>,
    /// Font for menu text.
    pub font: Handle<Font>,
    /// Font for menu text.
    pub font_size_scale: f32,

    pub text_shadow_color: Color,
    pub normal_button_color: Color,
    pub hovered_button_color: Color,
    pub pressed_button_color: Color,
}

impl MenuStyle {
    pub fn with_font(self, font: Handle<Font>) -> Self {
        Self { font, ..self }
    }
    pub fn with_font_size_scale(self, font_size_scale: f32) -> Self {
        Self {
            font_size_scale,
            ..self
        }
    }
    pub fn with_page_color(self, page_color: Option<Color>) -> Self {
        Self { page_color, ..self }
    }
    pub fn with_page_image(self, page_image: Option<Handle<Image>>) -> Self {
        Self { page_image, ..self }
    }
    pub fn with_text_shadow_color(self, text_shadow_color: Color) -> Self {
        Self {
            text_shadow_color,
            ..self
        }
    }
    pub fn with_normal_button_color(self, normal_button_color: Color) -> Self {
        Self {
            normal_button_color,
            ..self
        }
    }
    pub fn with_pressed_button_color(self, pressed_button_color: Color) -> Self {
        Self {
            pressed_button_color,
            ..self
        }
    }
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            // This are set in [set_default_menu_style]
            font: default(),
            font_size_scale: 1.0,

            page_color: Some(tailwind::BLUE_900.with_alpha(0.25).into()),
            page_image: None,
            text_shadow_color: Color::hsva(0.0, 0.0, 0.25, 1.0),
            normal_button_color: Color::hsva(0.0, 0.0, 0.75, 1.0),
            hovered_button_color: Color::hsva(0.0, 0.0, 1.0, 1.0),
            pressed_button_color: Color::srgb(0.35, 0.75, 0.35),
        }
    }
}

pub struct MenuCommonPlugin;
impl Plugin for MenuCommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TabNavigationPlugin)
            .insert_resource(MenuStyle::default())
            .insert_resource(DraggingMenuItem(None))
            .insert_resource(MenuItemSelectionHistory::default())
            .insert_resource(PreviousMenuStack::default())
            .add_message::<MenuActionMessage>()
            .add_systems(PreUpdate, on_uifont_init.run_if(resource_changed::<UiFont>))
            .add_systems(
                PreUpdate,
                // Common handling for any menu.
                (
                    on_added_menu_item,
                    on_focus_change,
                    handle_menu_action,
                    handle_menu_item_decoration,
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
                    .chain()
                    .run_if(is_in_menu),
            )
            .add_systems(Update, (handle_menu_back, handle_menu_into).chain());

        #[cfg(feature = "input_bei")]
        {
            app.add_observer(on_menu_back)
                .add_observer(on_menu_down_up)
                .add_observer(on_focused_reset)
                .add_observer(on_focused_interact_fire)
                .add_observer(on_focused_left_right);
        }
    }
}

fn on_uifont_init(ui_font: Res<UiFont>, mut menu_style: If<ResMut<MenuStyle>>) {
    if menu_style.font == Handle::default() {
        menu_style.font = ui_font.0.clone();
    }
}

#[derive(Resource)]
pub struct CountAccumulator<T> {
    dir: f32,
    count: f32,
    _marker: PhantomData<T>,
}

impl<T> Default for CountAccumulator<T> {
    fn default() -> Self {
        Self {
            dir: 0.,
            count: 0.,
            _marker: default(),
        }
    }
}

impl<T> CountAccumulator<T> {
    pub fn reset(&mut self) {
        self.count = 0.;
    }

    pub fn add_and_test(&mut self, delta: f32) -> Option<i32> {
        let was_zero = self.count == 0.;

        let intent = signum_or_zero(delta);
        if intent != signum_or_zero(self.dir) {
            self.count = 0.;
        }
        self.dir = delta;

        self.count += delta.abs();

        let overflowed = self.count >= 1.;
        if overflowed {
            // self.count = -self.count.fract();
            self.count = 0.001;
        }

        if (was_zero && self.count != 0.) || overflowed {
            Some(intent)
        } else {
            None
        }
    }
}

fn handle_menu_back(
    mut commands: Commands,
    _go_back_in_menu_request: If<Res<GoBackInMenuRequest>>,
    overlay_state: Res<State<OverlayState>>,
    pre_menu: Option<ResMut<OverlayStateBeforeMenu>>,
    mut prev_menu: ResMut<PreviousMenuStack>,
) {
    // Ok, pop something.
    if let Some(prev) = prev_menu.0.pop() {
        commands.set_state(prev);
    } else if *overlay_state.get() != OverlayState::MainMenu {
        // Done, no more history.
        let previous = if let Some(pre_menu) = pre_menu {
            pre_menu.0
        } else {
            OverlayState::Hidden
        };
        commands.remove_resource::<OverlayStateBeforeMenu>();
        commands.set_state(previous);
    }

    // Handled, whether or not it did anything.
    commands.remove_resource::<GoBackInMenuRequest>();
}

fn handle_menu_into(
    mut commands: Commands,
    go_into_in_menu_request: If<Res<GoIntoMenuRequest>>,
    overlay_state: Res<State<OverlayState>>,
    mut prev_menu: ResMut<PreviousMenuStack>,
) {
    let current = *overlay_state.get();
    let to_enter: OverlayState = ***go_into_in_menu_request;
    if current == to_enter {
        commands.remove_resource::<GoIntoMenuRequest>();
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
        warn!("not a menu: {to_enter:?}");
    }

    // Handled, whether or not it did anything.
    commands.remove_resource::<GoIntoMenuRequest>();
}

pub struct MenuItemBuilder<'w, 's> {
    page: Entity,
    overlay: OverlayState,
    program: ProgramState,
    style: MenuStyle,
    commands: Commands<'w, 's>,
    item_index: i32,
    previous_first_ent_label: Option<(Entity, String)>,
    first_ent_label: Option<(Entity, String)>,
}

impl<'w, 's> MenuItemBuilder<'w, 's> {
    pub fn new(
        commands: Commands<'w, 's>,
        overlay: OverlayState,
        program: ProgramState,
        style: &MenuStyle,
        history: &MenuItemSelectionHistory,
    ) -> Self {
        Self {
            overlay,
            program,
            style: (*style).clone(),
            page: Entity::PLACEHOLDER,
            commands,
            item_index: 0,
            previous_first_ent_label: history.0.get(&overlay).cloned(),
            first_ent_label: None,
        }
    }

    pub fn with_font(self, font: Handle<Font>) -> Self {
        Self {
            style: self.style.with_font(font),
            ..self
        }
    }
    pub fn with_font_size_scale(self, font_scale: f32) -> Self {
        Self {
            style: self.style.with_font_size_scale(font_scale),
            ..self
        }
    }
    pub fn with_page_color(self, page_color: Option<Color>) -> Self {
        Self {
            style: self.style.with_page_color(page_color),
            ..self
        }
    }

    /// Create the menu page, applying the desired styling.
    pub fn page(mut self) -> Self {
        self.commands.remove_resource::<LastMenuFocus>();
        let page = self
            .commands
            .spawn((
                DespawnOnExit(self.overlay),
                create_menu_page_node(&self.style),
                TabGroup::new(FxBuildHasher.hash_one(self.overlay) as _),
            ))
            .with_child(Node {
                margin: UiRect {
                    top: Val::Px(if self.program == ProgramState::LaunchMenu {
                        128.0
                    } else {
                        0.0
                    }),
                    ..default()
                },
                ..default()
            })
            .with_child((
                ZIndex(-5),
                FixedNode,
                Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ImageNode {
                    image: self
                        .style
                        .page_image
                        .clone()
                        .unwrap_or(bevy::image::TRANSPARENT_IMAGE_HANDLE),
                    image_mode: NodeImageMode::Auto,
                    visual_box: VisualBox::ContentBox,
                    ..default()
                },
            ))
            .id();

        Self { page, ..self }
    }

    pub fn commands(&mut self) -> Commands<'_, '_> {
        self.commands.reborrow()
    }

    pub fn add_item(
        &mut self,
        text: impl Into<String>,
        other_menu_items: impl Bundle,
        handler: impl MenuItemHandler + 'static,
    ) -> &mut Self {
        if self.page == Entity::PLACEHOLDER {
            panic!("need to call MenuItemBuilder.page()");
        }

        let text: String = text.into();
        let tab_index = TabIndex(self.item_index);
        let mut ent_commands = self.commands.entity(self.page);
        let mut ent_id = Entity::PLACEHOLDER;
        ent_commands.with_children(|builder| {
            ent_id = spawn_menu_button_bundle(
                builder,
                text.clone(),
                &self.style,
                tab_index,
                Arc::new(Mutex::new(handler)),
            );
            builder
                .commands()
                .entity(ent_id)
                .try_insert(Name::new(text.clone()))
                .try_insert(other_menu_items)
                .try_insert_if(AutoFocus, || self.item_index == 0);
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
        style: &MenuStyle,
        other_menu_items: impl Bundle,
    ) -> &mut Self {
        let text: String = text.into();
        let mut ent_commands = self.commands.entity(self.page);
        let mut ent_id = Entity::PLACEHOLDER;
        ent_commands.with_children(|builder| {
            ent_id = spawn_menu_label_bundle(builder, text.clone(), style);
            builder
                .commands()
                .entity(ent_id)
                .try_insert(Name::new(text.clone()))
                .try_insert(other_menu_items);
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
            history.0.insert(self.overlay, (first_ent, label).clone());
            self.commands
                .insert_resource(InputFocus::from_entity(first_ent));
            self.commands.insert_resource(RefreshMenu);
            self.commands.entity(first_ent).try_insert(Hovered(true));
            self.commands
                .write_message(MenuActionMessage::Navigate(first_ent));
        }
    }
}

/// Overlay state before entering a menu.
#[derive(Resource, Debug, Default, Clone)]
pub(crate) struct OverlayStateBeforeMenu(pub(crate) OverlayState);

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
#[derive(Resource, Debug, Default, Clone, Deref)]
pub struct GoIntoMenuRequest(pub OverlayState);

/////

/// Event firing a menu action.
///
/// For each, the Entity referenced is the one that has the action applied.
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
    pub count: Arc<dyn Fn() -> usize + Send + Sync + 'static>,
    /// Get the string for the value.
    pub display: Arc<dyn Fn(usize) -> String + Send + Sync + 'static>,
}

impl MenuEnum {
    /// `get`: System that fetches the model value then sets it in Self::current.
    /// `set`: System that applies the given value to the model.
    /// `count`: How many items are there? Given the return, [0..count()) are valid.
    /// `display`: Get the display value of the selected index.
    pub fn new(
        get: SystemId<In<Entity>>,
        set: SystemId<In<usize>, ()>,
        count: impl Fn() -> usize + Send + Sync + 'static,
        display: impl Fn(usize) -> String + Send + Sync + 'static,
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
    /// Current model value, cached as of creation.
    pub current: Option<f32>,
    /// System that fetches the value from the model and then sets it in MenuSlider::current.
    pub get: SystemId<In<Entity>, ()>,
    /// System that applies the given value to the model.
    pub set: SystemId<In<f32>, ()>,
    /// Get the default model value.
    pub default_fn: Arc<dyn Fn() -> Option<f32> + Send + Sync + 'static>,
    /// Convert the model value to UI.
    pub to_ui_fn: Arc<dyn Fn(f32) -> f32 + Send + Sync + 'static>,
    /// Convert the UI value to model.
    pub from_ui_fn: Arc<dyn Fn(f32) -> f32 + Send + Sync + 'static>,
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
        default: impl Fn() -> Option<f32> + Send + Sync + 'static,
        to_ui: impl Fn(f32) -> f32 + Send + Sync + 'static,
        from_ui: impl Fn(f32) -> f32 + Send + Sync + 'static,
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

    pub fn range(&self) -> f32 {
        *self.ui_range.end() - *self.ui_range.start()
    }

    pub fn add_and_clamp(&self, delta: f32, ui_orig: f32) -> f32 {
        let delta = if delta.abs() < self.ui_step_base {
            self.ui_step_base.copysign(delta)
        } else {
            (delta / self.ui_step_base).round() * self.ui_step_base
        };
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

fn create_menu_page_node(style: &MenuStyle) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(style.page_color.unwrap_or(Color::BLACK)),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),
    )
}

fn spawn_menu_button_bundle(
    builder: &mut ChildSpawnerCommands,
    text: String,
    style: &MenuStyle,
    tab_index: TabIndex,
    handler: Arc<Mutex<dyn MenuItemHandler>>,
) -> Entity {
    let ent_id = spawn_menu_row_bundle(builder, style);
    builder.commands().entity(ent_id).try_insert((
        Button,
        Hovered::default(),
        Text::new(text.clone()),
        MenuBaseText(text, style.font_size_scale),
        tab_index,
        MenuItem(handler),
        RelativeCursorPosition::default(),
    ));
    ent_id
}

fn spawn_menu_label_bundle(
    builder: &mut ChildSpawnerCommands,
    text: String,
    style: &MenuStyle,
) -> Entity {
    let ent_id = spawn_menu_row_bundle(builder, style);
    builder.commands().entity(ent_id).try_insert((
        Text::new(text.clone()),
        MenuBaseText(text, style.font_size_scale),
    ));
    ent_id
}

/// Add a menu item row to a given menu page.
fn spawn_menu_row_bundle(builder: &mut ChildSpawnerCommands, style: &MenuStyle) -> Entity {
    builder
        .spawn((
            Node {
                padding: UiRect::axes(MARGIN, MARGIN),
                align_self: AlignSelf::Center,
                height: (MENU_ITEM_FONT_SIZE * style.font_size_scale * 2.0).into(),
                ..default()
            },
            TextFont {
                font: style.font.clone().into(),
                font_size: (MENU_ITEM_FONT_SIZE * style.font_size_scale).into(),
                ..default()
            },
            LineHeight::RelativeToFont(style.font_size_scale),
            TextColor::WHITE,
            TextShadow {
                color: style.text_shadow_color,
                offset: Vec2::splat(SHADOW_OFFSET * style.font_size_scale),
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
                move |mut trigger: On<PointerPress>,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      slider_q: Query<&MenuSlider>,
                      mut dragging: ResMut<DraggingMenuItem>| {
                    if focus.get().is_none()
                        || slider_q.get(trigger.event().entity).iter().next().is_none()
                    {
                        focus.set(trigger.event().entity, FocusCause::Navigated);
                        visible.0 = true;
                        dragging.0 = None;
                        trigger.propagate(false);
                    }
                },
            )
            .observe(
                move |mut trigger: On<PointerClick>,
                      mut commands: Commands,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      slider_q: Query<&MenuSlider>,
                      dragging: Res<DraggingMenuItem>| {
                    let was_focused = focus.get() == Some(trigger.event().entity);
                    focus.set(trigger.event().entity, FocusCause::Pressed);
                    visible.0 = true;
                    if was_focused
                        && (dragging.0.is_none()
                            && (slider_q.get(trigger.event().entity).iter().next().is_none()
                                || trigger.event().duration < Duration::from_millis(100)))
                    {
                        commands.write_message(MenuActionMessage::Activate(trigger.event().entity));
                        trigger.propagate(false);
                    }
                },
            );

        if toggle {
            ent_commands.observe(
                move |mut trigger: On<PointerClick>,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      dragging: Option<Res<DraggingMenuItem>>,
                      mut writer: MessageWriter<MenuActionMessage>| {
                    focus.set(trigger.event().entity, FocusCause::Pressed);
                    visible.0 = true;
                    if dragging.is_none() {
                        writer.write(MenuActionMessage::Activate(trigger.event().entity));
                        trigger.propagate(false);
                    }
                },
            );
        }

        if enm {
            ent_commands.observe(
                move |mut trigger: On<PointerClick>,
                      mut focus: ResMut<InputFocus>,
                      mut visible: ResMut<InputFocusVisible>,
                      dragging: Option<Res<DraggingMenuItem>>,
                      mut writer: MessageWriter<MenuActionMessage>| {
                    focus.set(trigger.event().entity, FocusCause::Pressed);
                    visible.0 = true;
                    if dragging.is_none() {
                        writer.write(MenuActionMessage::Next(trigger.event().entity));
                        trigger.propagate(false);
                    }
                },
            );
        }

        if slider || enm {
            ent_commands
                .observe(
                    move |trigger: On<PointerDragStart>,
                          mut focus: ResMut<InputFocus>,
                          mut visible: ResMut<InputFocusVisible>,
                          mut dragging: ResMut<DraggingMenuItem>| {
                        focus.set(trigger.event().entity, FocusCause::Pressed);
                        visible.0 = true;
                        dragging.0 = Some(trigger.event().entity);
                    },
                )
                .observe(
                    move |trigger: On<PointerDrag>,
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
                    move |mut trigger: On<PointerDragEnd>,
                          mut dragging: ResMut<DraggingMenuItem>| {
                        dragging.0 = None;
                        trigger.propagate(false);
                    },
                );
        }
    }
}

/// The last item focused, to restore if we click away from focusable items.
#[derive(Resource, Debug, Reflect, Deref, DerefMut)]
#[reflect(Resource, Debug)]
#[type_path = "game"]
struct LastMenuFocus(Entity);

// HACK: clicking outside a menu item gives focus to nothing at all.
fn on_focus_change(
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
    prev_focus: Option<Res<LastMenuFocus>>,
    menu_item_q: Query<Entity, With<MenuItem>>,
    auto_focus_q: Query<&AutoFocus, With<MenuItem>>,
) {
    if !focus.is_changed() {
        return;
    }

    if focus.get().is_none() {
        debug!("lost focus");

        // Did we click off something?
        let mut auto = None;
        for ent in menu_item_q {
            if prev_focus.as_ref().is_some_and(|prev| ***prev == ent) {
                debug!("switch to previous {ent}");
                focus.set(ent, FocusCause::Navigated);
                return;
            }
            if auto.is_none() && auto_focus_q.contains(ent) {
                auto = Some(ent);
            }
        }

        // Nothing.
        if let Some(ent) = auto {
            debug!("switch to Auto {ent}");
            focus.set(ent, FocusCause::Navigated);
        }
    }

    if let Some(ent) = focus.get() {
        commands.insert_resource(LastMenuFocus(ent));
    }
}

#[cfg(feature = "input_bei")]
fn on_focused_interact_fire(
    _event: On<Start<actions::Interact>>,
    mut commands: Commands,

    focus: Res<InputFocus>,
    menu_item_q: Query<&MenuItem>,
) {
    let Some(entity) = focus.get() else { return };

    // Activate menu item?
    if menu_item_q.contains(entity) {
        commands.write_message(MenuActionMessage::Activate(entity));
    } else {
        // warn!("no MenuItem");
    }
}

#[cfg(feature = "input_bei")]
fn on_focused_reset(
    _event: On<Start<actions::Reset>>,
    mut commands: Commands,

    focus: Res<InputFocus>,
    menu_item_q: Query<&MenuItem>,
) {
    let Some(entity) = focus.get() else { return };

    if menu_item_q.contains(entity) {
        commands.write_message(MenuActionMessage::Reset(entity));
    } else {
        warn!("no MenuItem");
    }
}

#[cfg(feature = "input_bei")]
fn on_focused_left_right(
    event: On<Fire<actions::MoveRightLeft>>,
    overlay: Res<State<OverlayState>>,

    focus: Res<InputFocus>,
    toggle_q: Query<&MenuToggle>,
    slider_q: Query<&MenuSlider>,
    enum_q: Query<&MenuEnum>,
    mut writer: MessageWriter<MenuActionMessage>,
) {
    if !overlay.is_menu() {
        return;
    };
    let Some(entity) = focus.get() else { return };

    let dir = event.value.signum() as i32;
    if slider_q.contains(entity) {
        writer.write(MenuActionMessage::Slide(entity, dir as f32));
    } else if toggle_q.contains(entity) || enum_q.contains(entity) {
        if dir < 0 {
            writer.write(MenuActionMessage::Previous(entity));
        } else {
            writer.write(MenuActionMessage::Next(entity));
        }
    }
}

fn signum_or_zero(f: f32) -> i32 {
    if f < 0. {
        -1
    } else if f > 0. {
        1
    } else {
        0
    }
}

#[cfg(feature = "input_bei")]
fn on_menu_back(
    _event: On<Start<actions::Back>>,
    overlay: Res<State<OverlayState>>,
    mut commands: Commands,
) {
    if !overlay.is_menu() {
        return;
    };
    commands.insert_resource(GoBackInMenuRequest);
}

#[cfg(feature = "input_bei")]
fn on_menu_down_up(
    event: On<Fire<actions::MoveUpDown>>,
    overlay: Res<State<OverlayState>>,
    mut commands: Commands,
    nav: TabNavigation,

    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
) {
    if !overlay.is_menu() {
        return;
    };
    if focus.get().is_none() {
        return;
    };

    let dir = event.value.signum() as i32;
    // Move in menu?
    let Some(nav_dir) = (match dir {
        -1 => Some(NavAction::Previous),
        1 => Some(NavAction::Next),
        _ => return,
    }) else {
        return;
    };
    let maybe_next = nav.navigate(&focus, nav_dir);

    match maybe_next {
        Ok(next) => {
            focus.set(next, FocusCause::Navigated);
            visible.0 = true;
            commands.write_message(MenuActionMessage::Navigate(next));
        }
        Err(e) => {
            // This failure mode is recoverable, but still indicates a problem.
            // warn!("Tab navigation error: {}", e);
            if let TabNavigationError::NoTabGroupForCurrentFocus { new_focus, .. } = e {
                focus.set(new_focus, FocusCause::Navigated);
                visible.0 = true;
                commands.write_message(MenuActionMessage::Navigate(new_focus));
            }
        }
    }
}

#[derive(Resource)]
struct MouseSliderReleaseTime {
    ent: Entity,
    slider_release_time: Duration,
}

fn handle_menu_mouse_drag(
    mut commands: Commands,
    mut reader: MessageReader<PointerDrag>,
    mut dragging: ResMut<DraggingMenuItem>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut writer: MessageWriter<MenuActionMessage>,
    focus: Res<InputFocus>,
) {
    if let Some(focus) = focus.get().as_ref() {
        for event in reader.read() {
            writer.write(MenuActionMessage::Slide(
                *focus,
                event.distance.x / window.width(),
            ));
            dragging.0 = Some(*focus);
            commands.remove_resource::<MouseSliderReleaseTime>();
        }
    }
}

fn handle_menu_mouse_click(
    mut commands: Commands,
    int_q: Query<
        (Entity, Has<Pressed>, &Hovered),
        (Or<(Changed<Pressed>, Changed<Hovered>)>, With<MenuItem>),
    >,
    slider_q: Query<&MenuSlider>,
    time: Res<Time>,
    pressed: Option<Res<MouseSliderReleaseTime>>,
    mut focus: ResMut<InputFocus>,
) {
    // Delay before
    const DELAY: Duration = Duration::from_millis(250);
    for (ent, pressedf, hovered) in int_q.iter() {
        if pressedf {
            // Queue to activate menu item on release later if it's a slider.
            commands.insert_resource(MouseSliderReleaseTime {
                ent,
                slider_release_time: time.elapsed().saturating_add(DELAY),
            });
            focus.set(ent, FocusCause::Pressed);
        } else if hovered.get()
            && let Some(pressed) = &pressed
            && pressed.ent == ent
        {
            let should_fire =
                !slider_q.contains(ent) || time.elapsed() >= pressed.slider_release_time;
            if should_fire {
                commands.write_message(MenuActionMessage::Activate(ent));
                commands.remove_resource::<MouseSliderReleaseTime>();
            }
        }
    }
}

fn handle_menu_item_decoration(
    mut interaction_query: Query<
        (
            Entity,
            &Hovered,
            Has<Pressed>,
            &MenuBaseText,
            &mut TextFont,
            &mut TextColor,
        ),
        With<Button>,
    >,
    focus: Res<InputFocus>,
    style: Res<MenuStyle>,
) {
    for (ent, hovered, pressed, MenuBaseText(_, scale), mut text, mut color) in
        &mut interaction_query
    {
        let (font_size, item_color) = match (hovered.get(), pressed, focus.get() == Some(ent)) {
            (_, _, true) => (
                MENU_ITEM_FONT_SIZE * *scale * 1.1,
                style.pressed_button_color,
            ),
            (_, true, _) => (
                MENU_ITEM_FONT_SIZE * *scale * 1.1,
                style.pressed_button_color,
            ),
            (true, false, _) => (
                MENU_ITEM_FONT_SIZE * *scale * 1.1,
                style.hovered_button_color,
            ),
            (false, _, _) => (MENU_ITEM_FONT_SIZE * *scale, style.normal_button_color),
        };
        let font_size: FontSize = font_size.into();
        #[expect(clippy::float_cmp, reason = "binary diff checking")]
        if text.font_size != font_size || color.0 != item_color {
            text.font_size = font_size;
            color.0 = item_color;
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
        error!("failed to fetch menu action events");
        return;
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

        if let Ok(mut handler) = handler.lock() {
            handler.handle(world, &event);

            world.insert_resource(RefreshMenu);
        }
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
        Or<(Changed<MenuSlider>, Changed<MenuToggle>)>,
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
            MenuActionMessage::Navigate(_) | MenuActionMessage::Activate(_) => (),
            MenuActionMessage::Reset(_) => {
                if let Some(default) = (slider.default_fn)() {
                    // Reset.
                    commands.run_system_with(slider.set, default);
                    // Force refresh.
                    commands.run_system_with(slider.get, entity);
                }
            }
            MenuActionMessage::Slide(entity, delta) => {
                // Adjust value in UI space.
                let Some(current) = slider.current.as_mut() else {
                    error!("slider {entity} not initialized yet");
                    continue;
                };
                let current = *current;

                let orig = (slider.to_ui_fn)(current);
                let new = (slider.from_ui_fn)(slider.add_and_clamp(*delta, orig));

                commands.run_system_with(slider.set, new);
                // Refresh from internal value.
                commands.run_system_with(slider.get, *entity);
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

        if !matches!(event, MenuActionMessage::Activate(_)) {
            continue;
        }
        let Some(current) = toggle.current.as_mut() else {
            error!("toggle {entity} not initialized yet");
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
    mut enums_q: Query<(&mut Text, &MenuBaseText, &MenuEnum), Changed<MenuEnum>>,
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
            MenuActionMessage::Slide(_, distance) if distance.abs() >= 0.1 => {
                *slider_time = slider_time.saturating_sub(time.delta());
                if slider_time.is_zero() {
                    *slider_time = Duration::from_secs_f32(1.0 / 2.0);
                    distance.signum() as isize
                } else {
                    0
                }
            }
            MenuActionMessage::Next(_) => 1,
            MenuActionMessage::Previous(_) => -1,
            _ => 0,
        };
        if dir == 0 {
            continue;
        }

        // Adjust value in UI space.
        let Some(current) = enums.current.as_mut() else {
            error!("enum {entity} not initialized yet");
            continue;
        };
        let current = *current;
        let count = (enums.count)();

        let new = ((current as isize) + dir).rem_euclid(count as isize) as usize;

        // Set it.
        commands.run_system_with(enums.set, new);
        // Refresh from internal value once it happens.
        commands.run_system_with(enums.get, entity);
    }
}
