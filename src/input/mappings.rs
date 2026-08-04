/*
Davenstein - by David Petnick

Platform Gamepad Mapping Installation

GilRs Reads SDL_GAMECONTROLLERCONFIG When Bevy Constructs Its Gamepad Backend
These Mappings Must Therefore Be Installed Before DefaultPlugins Is Added
*/

#[cfg(target_os = "macos")]
const SDL_GAMECONTROLLERCONFIG: &str = "SDL_GAMECONTROLLERCONFIG";

// Keep Platform Mappings Together so Additional Retro USB Controllers Can Be
// Added Without Mixing Hardware Normalization Into Gameplay Input Processing
#[cfg(target_os = "macos")]
const MACOS_GAMEPAD_MAPPINGS: &[(&str, &str)] = &[
    (
        "03000000790000001100000006010000",
        concat!(
            "03000000790000001100000006010000,Retrolink SNES Controller,",
            "a:b1,b:b2,back:b8,leftshoulder:b4,lefty:a4,",
            "rightshoulder:b5,rightx:a3,start:b9,x:b3,y:b0,",
            "platform:Mac OS X,"
        ),
    ),
];

#[cfg(target_os = "macos")]
pub(crate) fn install_platform_gamepad_mappings() {
    let existing = match std::env::var(SDL_GAMECONTROLLERCONFIG) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => String::new(),
        // Do Not Destroy an Explicit Non-Unicode Environment Value
        Err(std::env::VarError::NotUnicode(_)) => return,
    };

    let mut merged = existing.clone();

    for &(guid, mapping) in MACOS_GAMEPAD_MAPPINGS {
        // Preserve an Explicit User or Launcher Mapping for the Same Hardware
        if merged.lines().any(|line| line.starts_with(guid)) {
            continue;
        }

        if !merged.is_empty() && !merged.ends_with('\n') {
            merged.push('\n');
        }

        merged.push_str(mapping);
    }

    if merged == existing {
        return;
    }

    // This Runs Before DefaultPlugins Constructs GilRs and Before Bevy Starts
    // Worker Threads so No Other Thread Can Read or Mutate the Environment Here
    unsafe {
        std::env::set_var(SDL_GAMECONTROLLERCONFIG, merged);
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn install_platform_gamepad_mappings() {}
