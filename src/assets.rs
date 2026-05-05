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

impl Plugin for CommonAssetsPlugin {
    fn build(&self, app: &mut App) {
        // See if we're in dev-land.
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            const COMMON_DIR: &str = "eds_bevy_common/assets";
            let mut comps = Path::new(&manifest_dir).ancestors();
            while let Some(test) = comps.next() {
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
#[allow(unused)]
pub struct CommonFxAssets {
    #[asset(path = "common://sounds/164472__deleted_user_2104797__crack-of-branch-3.ogg")]
    pub action: Handle<AudioSample>,

    #[asset(path = "common://sounds/03-197884__millavsb__elasticwhip.ogg")]
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

    #[asset(path = "common://sounds/00-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/01-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/02-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/03-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/04-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1f: Handle<AudioSample>,
    #[asset(path = "common://sounds/05-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1g: Handle<AudioSample>,
    #[asset(path = "common://sounds/06-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1h: Handle<AudioSample>,
    #[asset(path = "common://sounds/07-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1i: Handle<AudioSample>,
    #[asset(path = "common://sounds/08-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1j: Handle<AudioSample>,
    #[asset(path = "common://sounds/09-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1k: Handle<AudioSample>,
    #[asset(path = "common://sounds/10-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1l: Handle<AudioSample>,
    #[asset(path = "common://sounds/11-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1m: Handle<AudioSample>,
    #[asset(path = "common://sounds/12-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1n: Handle<AudioSample>,
    #[asset(path = "common://sounds/13-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1o: Handle<AudioSample>,
    #[asset(path = "common://sounds/14-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1p: Handle<AudioSample>,
    #[asset(path = "common://sounds/15-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1q: Handle<AudioSample>,
    #[asset(path = "common://sounds/16-366681__1san__elastic-bands-snapping.ogg")]
    pub snap1r: Handle<AudioSample>,

    #[asset(path = "common://sounds/01-655623__hankof__brush.ogg")]
    pub brush1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/02-655623__hankof__brush.ogg")]
    pub brush1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/03-655623__hankof__brush.ogg")]
    pub brush1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/04-655623__hankof__brush.ogg")]
    pub brush1d: Handle<AudioSample>,
    #[asset(path = "common://sounds/05-655623__hankof__brush.ogg")]
    pub brush1e: Handle<AudioSample>,
    #[asset(path = "common://sounds/06-655623__hankof__brush.ogg")]
    pub brush1f: Handle<AudioSample>,

    #[asset(path = "common://sounds/01-596484__eugeneeverett__planks-dropped.ogg")]
    pub wood1a: Handle<AudioSample>,
    #[asset(path = "common://sounds/02-596484__eugeneeverett__planks-dropped.ogg")]
    pub wood1b: Handle<AudioSample>,
    #[asset(path = "common://sounds/03-596484__eugeneeverett__planks-dropped.ogg")]
    pub wood1c: Handle<AudioSample>,
    #[asset(path = "common://sounds/04-596484__eugeneeverett__planks-dropped.ogg")]
    pub wood1d: Handle<AudioSample>,

    #[asset(path = "common://sounds/bump-629124__raygunv__spinning-top.ogg")]
    pub bump2: Handle<AudioSample>,
    #[asset(path = "common://sounds/412378__smokenweewalt__closet_hit_01.ogg")]
    pub bump3: Handle<AudioSample>,
}

#[cfg(feature = "midi_synth")]
#[derive(Resource, AssetCollection)]
#[allow(unused)]
pub struct CommonSoundFontAssets {
    #[asset(path = "common://soundfonts/TimGM6mb.sf2")]
    pub timgm6mb: Handle<SoundFont>,
}

#[derive(Resource, AssetCollection)]
#[allow(unused)]
pub struct CommonSkyboxAssets {
    #[asset(path = "common://skyboxes/dresden_station_night.exr")]
    pub dresden_station_night: Handle<Image>,
    #[asset(path = "common://skyboxes/farm_field_puresky_4k.exr")]
    pub farm_field_puresky: Handle<Image>,
    #[asset(path = "common://skyboxes/starmap_2020_2k.exr")]
    pub starmap_2020: Handle<Image>,
}
