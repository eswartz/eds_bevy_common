//! Common assets.
//!
//!
use std::path::Path;
use bevy::prelude::*;
use bevy::asset::io::AssetSourceBuilder;
use bevy_asset_loader::prelude::*;
use bevy_seedling::sample::AudioSample;
use crate::find_runtime_base_directory_by_folder;

#[cfg(feature = "midi_synth")]
use crate::midi_synth::asset::SoundFont;

pub struct CommonAssetsPlugin;

pub mod surfaces;

impl Plugin for CommonAssetsPlugin {
    fn build(&self, app: &mut App) {
        // See if we're in dev-land.
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            const COMMON_DIR: &str = "eds_bevy_common/assets";
            for test in Path::new(&manifest_dir).ancestors() {
                let common_assets = Path::new(&test).join(COMMON_DIR);
                if common_assets.is_dir() {
                    eprintln!("info: using {common_assets:?} for 'common' assets");
                    app.register_asset_source(
                        "common",
                        AssetSourceBuilder::platform_default(
                            &common_assets.display().to_string(),
                            None,
                        ),
                    );
                    return;
                }
            }

            // OK, did not find it. Do the uglier work of sniffing
            // around in the git repo checkout.
            if let Ok(cargo_dir) = std::env::var("CARGO_HOME") {
                let git_checkouts = Path::new(&cargo_dir).join("git").join("checkouts");
                if let Ok(dir) = git_checkouts.read_dir() {
                    let mut newest = None;
                    let mut newest_dir = None;
                    for ent in dir {
                        let Ok(ent) = ent else { continue };
                        let name = ent.file_name().display().to_string();
                        if name.starts_with("eds_bevy_common-") {
                            let exp_dir = ent.path();
                            eprintln!("info: searching within {exp_dir:?}");

                            // This holds subdirs named after checkout SHA1 prefixes.
                            if let Ok(revs) = exp_dir.read_dir() {
                                for rev in revs {
                                    let Ok(rev) = rev else { continue };
                                    let Ok(revm) = rev.metadata() else { continue };
                                    let Ok(revmt) = revm.modified() else { continue };
                                    if revm.is_dir() && newest.is_none_or(|n| revmt > n) {
                                        newest_dir = Some(rev.path());
                                        newest = Some(revmt);
                                    }
                                }
                            }

                        }
                    }
                    if let Some(newest_dir) = newest_dir {
                        eprintln!("info: using eds_bevy_common git repo checkout at {newest_dir:?}");
                        app.register_asset_source(
                            "common",
                            AssetSourceBuilder::platform_default(
                                &newest_dir.join("assets").display().to_string(),
                                None,
                            ),
                        );
                        return;
                    }
                }
            }

            log::error!("error: did not find eds_bevy_common git repo checkout");
        }

        // Available from cwd?
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(cwd) = std::env::current_dir() {
            let assets = cwd.join("../eds_bevy_common/assets");
            if assets.is_dir() {
                log::info!("Using {assets:?} for 'common' assets");
                app.register_asset_source(
                    "common",
                    AssetSourceBuilder::platform_default(
                        &assets.display().to_string(),
                        None,
                    ),
                );
                return;
            }
        }

        // Assets better be installed.
        if let Ok(base_dir) = find_runtime_base_directory_by_folder("assets") {
            log::info!("Using {base_dir:?} for 'common' assets");
            let assets = if cfg!(target_arch = "wasm32") {
                "assets".to_string()
            } else {
                base_dir.join("assets").display().to_string()
            };
            log::info!("adding common assets at {assets:?}");
            app.register_asset_source(
                "common",
                AssetSourceBuilder::platform_default(
                    &assets,
                    None,
                ),
            );
            return;
        }

        log::error!("error: did not find eds_bevy_common/assets");
    }
}

#[derive(Resource, AssetCollection)]
pub struct CommonGuiAssets {
    /// This font provides common icons (pause/mute).
    #[asset(path = "common://fonts/emoji-icon-font.ttf")]
    pub emoji_icon_font: Handle<Font>,
    #[asset(path = "common://fonts/Hack-Regular.ttf")]
    pub hack_font: Handle<Font>,

    #[asset(path = "common://fonts/Recursive-Bold.ttf")]
    pub std_ui: Handle<Font>,

    #[asset(path = "common://textures/crosshair.png")]
    pub crosshair: Handle<Image>,
    #[asset(path = "common://textures/crosshair_select.png")]
    pub crosshair_select: Handle<Image>,

    #[asset(path = "common://textures/power.png")]
    pub power_bar: Handle<Image>,
}

#[derive(Resource, AssetCollection)]
pub struct CommonFxAssets {
    #[asset(path = "common://sounds/164472__deleted_user_2104797__crack-of-branch-3.ogg")]
    pub action: Handle<AudioSample>,

    #[asset(path = "common://sounds/197884__millavsb__elasticwhip-03.ogg")]
    pub select: Handle<AudioSample>,
    #[asset(path = "common://sounds/164472__deleted_user_2104797__crack-of-branch-3-rev.ogg")]
    pub deselect: Handle<AudioSample>,

    #[asset(path = "common://sounds/257803__xtrgamr__swish-2_swish-178056__eneasz__folder-snapped-shut.ogg")]
    pub swoosh: Handle<AudioSample>,
    #[asset(path = "common://sounds/414763__michorvath__click.ogg")]
    pub bump0a: Handle<AudioSample>,
    #[asset(path = "common://sounds/496760__malle99__click-tick-2.ogg")]
    pub bump0b: Handle<AudioSample>,
    #[asset(path = "common://sounds/384187__malle99__click-tick.ogg")]
    pub bump0c: Handle<AudioSample>,
    #[asset(path = "common://sounds/tiny-487531__ranner__bubble-short.ogg")]
    pub bump1a: Handle<AudioSample>,

    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-00.ogg")]
    pub snap1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-01.ogg")]
    pub snap1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-02.ogg")]
    pub snap1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-03.ogg")]
    pub snap1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-04.ogg")]
    pub snap1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-05.ogg")]
    pub snap1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-06.ogg")]
    pub snap1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-07.ogg")]
    pub snap1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-08.ogg")]
    pub snap1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-09.ogg")]
    pub snap1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-10.ogg")]
    pub snap1l: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-11.ogg")]
    pub snap1m: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-12.ogg")]
    pub snap1n: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-13.ogg")]
    pub snap1o: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-14.ogg")]
    pub snap1p: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-15.ogg")]
    pub snap1q: Handle<AudioSample>,
    #[asset(path = "common://sounds/366681__1san__elastic-bands-snapping-16.ogg")]
    pub snap1r: Handle<AudioSample>,

    #[asset(path = "common://sounds/655623__hankof__brush-01.ogg")]
    pub brush1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/655623__hankof__brush-02.ogg")]
    pub brush1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/655623__hankof__brush-03.ogg")]
    pub brush1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/655623__hankof__brush-04.ogg")]
    pub brush1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/655623__hankof__brush-05.ogg")]
    pub brush1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/655623__hankof__brush-06.ogg")]
    pub brush1f: Handle<AudioSample>,

    #[asset(path = "common://sounds/596484__eugeneeverett__planks-dropped-01.ogg")]
    pub wood1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/596484__eugeneeverett__planks-dropped-02.ogg")]
    pub wood1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/596484__eugeneeverett__planks-dropped-03.ogg")]
    pub wood1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/596484__eugeneeverett__planks-dropped-04.ogg")]
    pub wood1d: Handle<AudioSample>,

    #[asset(path = "common://sounds/bump-629124__raygunv__spinning-top.ogg")]
    pub bump2: Handle<AudioSample>,
    #[asset(path = "common://sounds/412378__smokenweewalt__closet_hit_01.ogg")]
    pub bump3: Handle<AudioSample>,

    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-01.ogg")]
    pub glass_scrape1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-02.ogg")]
    pub glass_scrape1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-03.ogg")]
    pub glass_scrape1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-04.ogg")]
    pub glass_scrape1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-05.ogg")]
    pub glass_scrape1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-06.ogg")]
    pub glass_scrape1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-07.ogg")]
    pub glass_scrape1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-08.ogg")]
    pub glass_scrape1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/800117__cvltiv8r__shells-and-quartz-crystals-rustling-and-scraping-on-marble-09.ogg")]
    pub glass_scrape1i: Handle<AudioSample>,

    #[asset(path = "common://sounds/638927__mfj0__metal-clanging-noises-01.ogg")]
    pub metal_clang_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/638927__mfj0__metal-clanging-noises-02.ogg")]
    pub metal_clang_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/638927__mfj0__metal-clanging-noises-03.ogg")]
    pub metal_clang_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/638927__mfj0__metal-clanging-noises-04.ogg")]
    pub metal_clang_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/638927__mfj0__metal-clanging-noises-05.ogg")]
    pub metal_clang_1e: Handle<AudioSample>,

    #[asset(path = "common://sounds/340915__passairmangrace__metalclang1_loud_bip-01.ogg")]
    pub metal_clang_2a: Handle<AudioSample>,
    #[asset(path = "common://sounds/340915__passairmangrace__metalclang1_loud_bip-02.ogg")]
    pub metal_clang_2b: Handle<AudioSample>,

    #[asset(path = "common://sounds/842171__aardsreal__basic-metal-clang-free.ogg")]
    pub metal_clang_3a: Handle<AudioSample>,

    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-01.ogg")]
    // slide
    pub earth_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-02.ogg")]
    pub earth_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-03.ogg")]
    pub earth_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-04.ogg")]
    pub earth_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-05.ogg")]
    pub earth_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-06.ogg")]
    pub earth_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-07.ogg")]
    // noise
    pub earth_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-08.ogg")]
    // noise
    pub earth_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/584891__krystianpawlowski__big-log-on-dirt-throwing-09.ogg")]
    // noise
    pub earth_1i: Handle<AudioSample>,

    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-01.ogg")]
    pub stone_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-02.ogg")]
    pub stone_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-03.ogg")]
    pub stone_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-04.ogg")]
    pub stone_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-05.ogg")]
    pub stone_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-06.ogg")]
    pub stone_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-07.ogg")]
    pub stone_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-08.ogg")]
    pub stone_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-09.ogg")]
    pub stone_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/545562__rsellick__kettlebell-concrete-drag-metal-rock-stone-earth-10.ogg")]
    pub stone_1j: Handle<AudioSample>,

    #[asset(path = "common://sounds/221954__kurono01__rock-friction-01.ogg")]
    pub stone_2a: Handle<AudioSample>,
    #[asset(path = "common://sounds/221954__kurono01__rock-friction-02.ogg")]
    pub stone_2b: Handle<AudioSample>,
    #[asset(path = "common://sounds/221954__kurono01__rock-friction-03.ogg")]
    pub stone_2c: Handle<AudioSample>,
    #[asset(path = "common://sounds/221954__kurono01__rock-friction-04.ogg")]
    pub stone_2d: Handle<AudioSample>,
    #[asset(path = "common://sounds/221954__kurono01__rock-friction-05.ogg")]
    pub stone_2e: Handle<AudioSample>,
    #[asset(path = "common://sounds/221954__kurono01__rock-friction-06.ogg")]
    pub stone_2f: Handle<AudioSample>,

    #[asset(path = "common://sounds/567701__nox_sound__foley_rocks_stones_impacts_mono-01.ogg")]
    pub rock_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/567701__nox_sound__foley_rocks_stones_impacts_mono-02.ogg")]
    pub rock_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/567701__nox_sound__foley_rocks_stones_impacts_mono-03.ogg")]
    pub rock_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/567701__nox_sound__foley_rocks_stones_impacts_mono-04.ogg")]
    pub rock_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/567701__nox_sound__foley_rocks_stones_impacts_mono-05.ogg")]
    pub rock_1e: Handle<AudioSample>,

    #[asset(path = "common://sounds/437357__giddster__handling-rocks-01.ogg")]
    pub rock_2a: Handle<AudioSample>,
    #[asset(path = "common://sounds/437357__giddster__handling-rocks-02.ogg")]
    pub rock_2b: Handle<AudioSample>,
    #[asset(path = "common://sounds/437357__giddster__handling-rocks-03.ogg")]
    pub rock_2c: Handle<AudioSample>,
    #[asset(path = "common://sounds/437357__giddster__handling-rocks-04.ogg")]
    pub rock_2d: Handle<AudioSample>,

    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-01.ogg")]
    pub footsteps_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-02.ogg")]
    pub footsteps_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-03.ogg")]
    pub footsteps_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-04.ogg")]
    pub footsteps_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-05.ogg")]
    pub footsteps_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-06.ogg")]
    pub footsteps_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-07.ogg")]
    pub footsteps_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-08.ogg")]
    pub footsteps_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-09.ogg")]
    pub footsteps_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-10.ogg")]
    pub footsteps_1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-11.ogg")]
    pub footsteps_1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-12.ogg")]
    pub footsteps_1l: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-13.ogg")]
    pub footsteps_1m: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-14.ogg")]
    pub footsteps_1n: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-15.ogg")]
    pub footsteps_1o: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-16.ogg")]
    pub footsteps_1p: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-17.ogg")]
    pub footsteps_1q: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-18.ogg")]
    pub footsteps_1r: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-19.ogg")]
    pub footsteps_1s: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-20.ogg")]
    pub footsteps_1t: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-21.ogg")]
    pub footsteps_1u: Handle<AudioSample>,
    #[asset(path = "common://sounds/462975__primisteka__pasos-rapidos-loseta-22.ogg")]
    pub footsteps_1v: Handle<AudioSample>,

    #[asset(path = "common://sounds/623692__launchsite__sandbag-drag-01.ogg")]
    pub sand_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/623692__launchsite__sandbag-drag-02.ogg")]
    pub sand_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/623692__launchsite__sandbag-drag-03.ogg")]
    pub sand_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/623692__launchsite__sandbag-drag-06.ogg")]
    pub sand_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/623692__launchsite__sandbag-drag-09.ogg")]
    pub sand_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/623692__launchsite__sandbag-drag-10.ogg")]
    pub sand_1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/623692__launchsite__sandbag-drag-11.ogg")]
    pub sand_1k: Handle<AudioSample>,

    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-01.ogg")]
    pub gravel_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-02.ogg")]
    pub gravel_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-03.ogg")]
    pub gravel_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-04.ogg")]
    pub gravel_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-05.ogg")]
    pub gravel_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-06.ogg")]
    pub gravel_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-07.ogg")]
    pub gravel_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-08.ogg")]
    pub gravel_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-09.ogg")]
    pub gravel_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-10.ogg")]
    pub gravel_1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-11.ogg")]
    pub gravel_1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/534917__lucas_schacht__walking-on-gravel-12.ogg")]
    pub gravel_1l: Handle<AudioSample>,

    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-01.ogg")]
    pub glass_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-02.ogg")]
    pub glass_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-03.ogg")]
    pub glass_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-04.ogg")]
    pub glass_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-05.ogg")]
    pub glass_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-06.ogg")]
    pub glass_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-07.ogg")]
    pub glass_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-08.ogg")]
    pub glass_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-09.ogg")]
    pub glass_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-10.ogg")]
    pub glass_1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-11.ogg")]
    pub glass_1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-12.ogg")]
    pub glass_1l: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-13.ogg")]
    pub glass_1m: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-14.ogg")]
    pub glass_1n: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-15.ogg")]
    pub glass_1o: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-16.ogg")]
    pub glass_1p: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-17.ogg")]
    pub glass_1q: Handle<AudioSample>,
    #[asset(path = "common://sounds/197563__dheming__glass_bottles_02-18.ogg")]
    pub glass_1r: Handle<AudioSample>,

    #[asset(path = "common://sounds/524498__krizin__pool-water-01.ogg")]
    pub water_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-02.ogg")]
    pub water_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-03.ogg")]
    pub water_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-04.ogg")]
    pub water_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-05.ogg")]
    pub water_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-06.ogg")]
    pub water_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-07.ogg")]
    pub water_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-08.ogg")]
    pub water_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-09.ogg")]
    pub water_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-10.ogg")]
    pub water_1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-11.ogg")]
    pub water_1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/524498__krizin__pool-water-12.ogg")]
    pub water_1l: Handle<AudioSample>,

    #[asset(path = "common://sounds/363186__littleluigi__1-thump-2.ogg")]
    pub thump_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/257803__xtrgamr__swish-2.ogg")]
    pub thump_2a: Handle<AudioSample>,
    #[asset(path = "common://sounds/bump-629124__raygunv__spinning-top.ogg")]
    pub thump_3a: Handle<AudioSample>,
    #[asset(path = "common://sounds/412378__smokenweewalt__closet_hit_01.ogg")]
    pub thump_4a: Handle<AudioSample>,
    #[asset(path = "common://sounds/466351__harrisando__thump.ogg")]
    pub thump_5a: Handle<AudioSample>,
    #[asset(path = "common://sounds/678491__adamcreeper__thump.ogg")]
    pub thump_6a: Handle<AudioSample>,
    #[asset(path = "common://sounds/344152__brokenphono__thump_003.ogg")]
    pub thump_7a: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-01.ogg")]
    pub thump_8a: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-02.ogg")]
    pub thump_8b: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-03.ogg")]
    pub thump_8c: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-04.ogg")]
    pub thump_8d: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-05.ogg")]
    pub thump_8e: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-06.ogg")]
    pub thump_8f: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-07.ogg")]
    pub thump_8g: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-08.ogg")]
    pub thump_8h: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-09.ogg")]
    pub thump_8i: Handle<AudioSample>,
    #[asset(path = "common://sounds/669883__snowfightstudios__various-thuds-10.ogg")]
    pub thump_8j: Handle<AudioSample>,

    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-01.ogg")]
    pub footsteps_2a: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-02.ogg")]
    pub footsteps_2b: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-03.ogg")]
    pub footsteps_2c: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-04.ogg")]
    pub footsteps_2d: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-05.ogg")]
    pub footsteps_2e: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-06.ogg")]
    pub footsteps_2f: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-07.ogg")]
    pub footsteps_2g: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-08.ogg")]
    pub footsteps_2h: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-09.ogg")]
    pub footsteps_2i: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-10.ogg")]
    pub footsteps_2j: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-11.ogg")]
    pub footsteps_2k: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-12.ogg")]
    pub footsteps_2l: Handle<AudioSample>,
    #[asset(path = "common://sounds/150444__splicesound__footsteps-solid-wood-rug-male-boots-medium-pace-and-scuffs-13.ogg")]
    pub footsteps_2m: Handle<AudioSample>,

    #[asset(path = "common://sounds/walking_gravel_1-01.ogg")]
    pub footsteps_3a: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-02.ogg")]
    pub footsteps_3b: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-03.ogg")]
    pub footsteps_3c: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-04.ogg")]
    pub footsteps_3d: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-05.ogg")]
    pub footsteps_3e: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-06.ogg")]
    pub footsteps_3f: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-07.ogg")]
    pub footsteps_3g: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-08.ogg")]
    pub footsteps_3h: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-09.ogg")]
    pub footsteps_3i: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_1-10.ogg")]
    pub footsteps_3j: Handle<AudioSample>,

    #[asset(path = "common://sounds/walking_gravel_3-01.ogg")]
    pub footsteps_4a: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_3-02.ogg")]
    pub footsteps_4b: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_3-03.ogg")]
    pub footsteps_4c: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_3-04.ogg")]
    pub footsteps_4d: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_3-05.ogg")]
    pub footsteps_4e: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_3-06.ogg")]
    pub footsteps_4f: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_3-07.ogg")]
    pub footsteps_4g: Handle<AudioSample>,
    #[asset(path = "common://sounds/walking_gravel_3-08.ogg")]
    pub footsteps_4h: Handle<AudioSample>,

    #[asset(path = "common://sounds/405667__anthousai__metal-bowl-handling-and-sliding-around-on-counter-01-01.ogg")]
    pub metal_bowl_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/405667__anthousai__metal-bowl-handling-and-sliding-around-on-counter-01-02.ogg")]
    pub metal_bowl_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/405667__anthousai__metal-bowl-handling-and-sliding-around-on-counter-01-03.ogg")]
    pub metal_bowl_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/405667__anthousai__metal-bowl-handling-and-sliding-around-on-counter-01-04.ogg")]
    pub metal_bowl_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/405667__anthousai__metal-bowl-handling-and-sliding-around-on-counter-01-05.ogg")]
    pub metal_bowl_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/405667__anthousai__metal-bowl-handling-and-sliding-around-on-counter-01-06.ogg")]
    pub metal_bowl_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/405667__anthousai__metal-bowl-handling-and-sliding-around-on-counter-01-07.ogg")]
    pub metal_bowl_1g: Handle<AudioSample>,

    #[asset(path = "common://sounds/448418__lordforklift__metal-slide-2-01.ogg")]
    pub metal_slide_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/448418__lordforklift__metal-slide-2-02.ogg")]
    pub metal_slide_1b: Handle<AudioSample>,

    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-01.ogg")]
    pub metal_hammer_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-02.ogg")]
    pub metal_hammer_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-03.ogg")]
    pub metal_hammer_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-04.ogg")]
    pub metal_hammer_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-05.ogg")]
    pub metal_hammer_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-06.ogg")]
    pub metal_hammer_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-07.ogg")]
    pub metal_hammer_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-08.ogg")]
    pub metal_hammer_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-09.ogg")]
    pub metal_hammer_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/383728__deleted_user_7146007__hitting-metal-hammering-slide-hammer-10.ogg")]
    pub metal_hammer_1j: Handle<AudioSample>,

    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-01.ogg")]
    pub wood_metal_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-02.ogg")]
    pub wood_metal_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-03.ogg")]
    pub wood_metal_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-04.ogg")]
    pub wood_metal_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-05.ogg")]
    pub wood_metal_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-06.ogg")]
    pub wood_metal_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-07.ogg")]
    pub wood_metal_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-08.ogg")]
    pub wood_metal_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-09.ogg")]
    pub wood_metal_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-10.ogg")]
    pub wood_metal_1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-11.ogg")]
    pub wood_metal_1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-12.ogg")]
    pub wood_metal_1l: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-13.ogg")]
    pub wood_metal_1m: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-14.ogg")]
    pub wood_metal_1n: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-15.ogg")]
    pub wood_metal_1o: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-16.ogg")]
    pub wood_metal_1p: Handle<AudioSample>,
    #[asset(path = "common://sounds/368654__dynamique__metal-object-sliding-on-wood-surface-17.ogg")]
    pub wood_metal_1q: Handle<AudioSample>,

    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-01.ogg")]
    pub metal_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-02.ogg")]
    pub metal_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-03.ogg")]
    pub metal_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-04.ogg")]
    pub metal_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-05.ogg")]
    pub metal_1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-06.ogg")]
    pub metal_1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-07.ogg")]
    pub metal_1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-08.ogg")]
    pub metal_1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-09.ogg")]
    pub metal_1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-10.ogg")]
    pub metal_1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-11.ogg")]
    pub metal_1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-12.ogg")]
    pub metal_1l: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-13.ogg")]
    pub metal_1m: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-14.ogg")]
    pub metal_1n: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-15.ogg")]
    pub metal_1o: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-16.ogg")]
    pub metal_1p: Handle<AudioSample>,
    #[asset(path = "common://sounds/628983__marsounds__knife-and-pot-sounds_sliding-hitting-17.ogg")]
    pub metal_1q: Handle<AudioSample>,

    #[asset(path = "common://sounds/148169__bhweber__glass_on_bar-01.ogg")]
    pub glass_roll_1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/148169__bhweber__glass_on_bar-02.ogg")]
    pub glass_roll_1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/148169__bhweber__glass_on_bar-03.ogg")]
    pub glass_roll_1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/148169__bhweber__glass_on_bar-04.ogg")]
    pub glass_roll_1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/148169__bhweber__glass_on_bar-05.ogg")]
    pub glass_roll_1e: Handle<AudioSample>,

    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-01.ogg")]
    pub footsteps_5a: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-02.ogg")]
    pub footsteps_5b: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-03.ogg")]
    pub footsteps_5c: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-04.ogg")]
    pub footsteps_5d: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-05.ogg")]
    pub footsteps_5e: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-06.ogg")]
    pub footsteps_5f: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-07.ogg")]
    pub footsteps_5g: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-08.ogg")]
    pub footsteps_5h: Handle<AudioSample>,
    #[asset(path = "common://sounds/583064__profispiesser__fx-sasc-heavy-impact-stone-glass-tiles-jump-walk-footstep-09.ogg")]
    pub footsteps_5i: Handle<AudioSample>,

    #[asset(path = "common://sounds/369071__lazylids__walking_on_broken_mirror-01.ogg")]
    pub footsteps_6a: Handle<AudioSample>,
    #[asset(path = "common://sounds/369071__lazylids__walking_on_broken_mirror-02.ogg")]
    pub footsteps_6b: Handle<AudioSample>,
    #[asset(path = "common://sounds/369071__lazylids__walking_on_broken_mirror-03.ogg")]
    pub footsteps_6c: Handle<AudioSample>,
    #[asset(path = "common://sounds/369071__lazylids__walking_on_broken_mirror-04.ogg")]
    pub footsteps_6d: Handle<AudioSample>,

}

#[cfg(feature = "midi_synth")]
#[derive(Resource, AssetCollection)]
pub struct CommonSoundFontAssets {
    #[asset(path = "common://soundfonts/TimGM6mb.sf2")]
    pub timgm6mb: Handle<SoundFont>,
}

#[derive(Resource, AssetCollection)]
pub struct CommonSkyboxAssets {
    #[asset(path = "common://skyboxes/dresden_station_night.exr")]
    pub dresden_station_night: Handle<Image>,
    #[asset(path = "common://skyboxes/farm_field_puresky_4k.exr")]
    pub farm_field_puresky: Handle<Image>,
    #[asset(path = "common://skyboxes/starmap_2020_2k.exr")]
    pub starmap_2020: Handle<Image>,
}
