
# Ed's Bevy Common

## Who's this for?

Me (`eswartz75`)!

## Should you care?

If you came across this repo, 👋.

Feel free to snoop around and steal ideas, even the bad ones.

But ⚠️⚠️⚠️ this is my personal repo and will <span style="color:red"> **change without warning** </span> ⚠️⚠️⚠️.

## What?

This is an *opinionated* set of plugins that can be used in Bevy game projects (it's not on crates.io). I've got a certain  I've been using this in my own (private) projects since Bevy 0.15. It currently works for 0.18.1.

*It is not documented* but just start from the example `example_menu.rs`.

But it contains:

* Mouse cursor grab/ungrab tracking

* Bevy `States` for program, gameplay, level progression

* Bevy `States` for overlay (2D) handling
** I.e. when a given menu is up, or whether debug UI is up
** Allows for cleanly hooking up setup/teardown/level switch

* Menuing framework (using `bevy_ui`)
** toggles
** sliders
** enums

* Play/pause support (`PauseState`, distinct gameplay and menu flags; pausing only affects `Time<Physics>`)
* `bevy_inspector_egui` integration
** `OverlayState::DebugGuiVisible` state used to avoid having debug UI clash with menus

* `bevy_asset_loader` integration with the states
** I.e. `New`/`Initializing` -> `AssetsLoaded` transitions
** I.e. so you know when you have fonts/icons/etc. available to use
** Any loading errors trigger `OverlayState::ErrorScreen` so users aren't left wondering when loading will end.

* player camera and FPS or space/flying controller
** keyboard/mouse integration so far
** support for animated "manual" camera control sections
* common action setup using leafwing-input-manager (`UserActions`)
* skybox and reflection probe setup from .exr files which occurs when asset collections are ready
and performs conversions into cubemaps

## Why?

* Obligation 😼: I used a version `v0.1` of this code for my Bevy Game Jam 7 project [he](https://github.com/eswartz/bevy-game-jam-7)[re](https://github.com/eswartz/bevy-game-jam-7-ghpages), and according to the rules, prewritten code for that project should be publically available.

* Benevolent indifference 🤷: I thought the code was good enough to *be* public, or even be snarfed into training data, and might be useful. Take it or leave it. Nothing here is revolutionary. Some things are bad. Some things are my reluctance to refactor.

* Pragmatism: Being public, I can link to it, share it amongst my projects without needing to have ssh credentials in random OSess and VMs.

I will continue to update this code and use that game jam entry for testing when I remember to. But I don't recommend linking to this crate without the caveat, beware of sudden changes.

## Attributions

* `fps.rs`: based on `bevy_mini_fps` (single-file implementation in `lib.rs`)
