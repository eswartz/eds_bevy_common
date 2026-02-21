
# Ed's Bevy Common

## What?

This is an *opinionated* set of plugins that can be used in Bevy game projects. I've been munging this in my own projects since Bevy 0.15.

It is mainly for my own personal projects and not aimed for widespread use just yet (though feel free to try), which is why it's in a repo and not on crates.io. But feel free to steal and adapt for your purposes.

## Why?

I'm making it available to help get started with:

* Mouse cursor grab/ungrab tracking

* Bevy `States` for program, gameplay, level progression

* Bevy `States` for overlay (2D) handling
** i.e. when a given menu is up, or whether debug UI is up
** Allows for cleanly hooking up setup/teardown/level switch

* Menu UI system until bevy_feathers gets us there
** toggles
** sliders
** enums

* Play/pause support (`PauseState`, pausing only affects `Time<Physics>`)
* `bevy_inspector_egui` integration
** `OverlayState::DebugGuiVisible` state used to avoid having debug UI clash with menus

* `bevy_asset_loader` integration with the states
** I.e. `New`/`Initializing` -> `AssetsLoaded` transitions
** I.e. so you know when you have fonts/icons/etc. available to use
** Any loading errors trigger `OverlayState::ErrorScreen` so users aren't left wondering when loading will end

* player camera and FPS or space/flying controller
** keyboard/mouse integration so far
** support for animated "manual" camera control sections
* common action setup using leafwing-input-manager (`UserActions`)
* skybox and reflection probe setup from .exr files which occurs when asset collections are ready
and performs conversions into cubemaps
*
