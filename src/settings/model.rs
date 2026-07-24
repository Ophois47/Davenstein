/*
Davenstein - by David Petnick

Settings Model - Serializable On-Disk Representation of Player Options

Mirrors the save/ Module's Layered Design: This File Is the Serializable DTO and
the Translation To and From the Live 'davelib::options' Resources. Like
save/model.rs, It Keeps serde Off the Engine Types (Which Live in davelib and
Carry No serde Dependency) and Instead Uses a Plain DTO in the Binary Crate.

Two Robustness Choices Worth Noting:
- Every DTO Field Is Optional, so a Missing or Older File Leaves That Setting at
  Its Live Default Rather Than Zeroing It. Only Fields That Are Present Overwrite.
- Enums Are Stored as Stable Lowercase Strings, so Reordering or Adding Engine
  Enum Variants Can Never Corrupt an Existing Config (Same Reasoning as the
  Explicit u8 Maps in save/capture.rs).

Key Bindings Are Intentionally Not Persisted Yet. They Hold bevy 'KeyCode' Values,
Which Need Either bevy's Optional 'serialize' Feature or an Explicit
KeyCode <-> String Map; That Lands in a Follow-Up Once the Approach Is Chosen.
Until Then Bindings Stay at Their Defaults on Load and Everything Else Persists.
*/

use serde::{Deserialize, Serialize};

use davelib::options::{
    ControlSettings,
    DisplayMode,
    GameplaySettings,
    MsaaSetting,
    RenderScale,
    SoundSettings,
    VideoSettings,
};

/// Bump When the On-Disk Settings Format Changes Incompatibly. A File With a
/// Different Version Is Ignored on Load (the Player Keeps Live Defaults)
pub const SETTINGS_FORMAT_VERSION: u32 = 1;

/// Top-Level Settings File. 'serde(default)' Makes Every Missing Field Fall Back
/// to Default (None for the Optionals, 0 for version) so Partial or Older Files
/// Never Fail to Parse
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SettingsFile {
    pub version: u32,
    pub video: VideoDto,
    pub control: ControlDto,
    pub sound: SoundDto,
    pub gameplay: GameplayDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct VideoDto {
    pub vsync: Option<bool>,
    /// "windowed" | "borderless" | "exclusive"
    pub display_mode: Option<String>,
    pub resolution: Option<[u32; 2]>,
    pub fov: Option<f32>,
    pub view_size: Option<u8>,
    /// "off" | "x4"
    pub msaa: Option<String>,
    /// "native" | "75" | "50" | "33"
    pub render_scale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ControlDto {
    pub mouse_sensitivity: Option<f32>,
    pub invert_y: Option<bool>,
    pub mouselook_enabled: Option<bool>,
    pub gamepad_enabled: Option<bool>,
    pub gamepad_sensitivity: Option<f32>,
    pub gamepad_deadzone: Option<f32>,
    // Key Bindings Deliberately Omitted for Now (See Module Header)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SoundDto {
    pub master_volume: Option<f32>,
    pub music_volume: Option<f32>,
    pub sfx_volume: Option<f32>,
    pub music_enabled: Option<bool>,
    pub sfx_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GameplayDto {
    pub reversible_pushwalls: Option<bool>,
}

// ---- Stable Enum <-> String Maps (Reorder-Proof) ----

fn display_mode_to_str(m: DisplayMode) -> &'static str {
    match m {
        DisplayMode::Windowed => "windowed",
        DisplayMode::BorderlessFullscreen => "borderless",
        DisplayMode::ExclusiveFullscreen => "exclusive",
    }
}

fn display_mode_from_str(s: &str) -> Option<DisplayMode> {
    match s {
        "windowed" => Some(DisplayMode::Windowed),
        "borderless" => Some(DisplayMode::BorderlessFullscreen),
        "exclusive" => Some(DisplayMode::ExclusiveFullscreen),
        _ => None,
    }
}

fn msaa_to_str(m: MsaaSetting) -> &'static str {
    match m {
        MsaaSetting::Off => "off",
        MsaaSetting::Sample4 => "x4",
    }
}

fn msaa_from_str(s: &str) -> Option<MsaaSetting> {
    match s {
        "off" => Some(MsaaSetting::Off),
        "x4" => Some(MsaaSetting::Sample4),
        _ => None,
    }
}

fn render_scale_to_str(r: RenderScale) -> &'static str {
    match r {
        RenderScale::Native => "native",
        RenderScale::Pct75 => "75",
        RenderScale::Pct50 => "50",
        RenderScale::Pct33 => "33",
    }
}

fn render_scale_from_str(s: &str) -> Option<RenderScale> {
    match s {
        "native" => Some(RenderScale::Native),
        "75" => Some(RenderScale::Pct75),
        "50" => Some(RenderScale::Pct50),
        "33" => Some(RenderScale::Pct33),
        _ => None,
    }
}

impl SettingsFile {
    /// Capture the Current Live Resources Into a Fully-Populated DTO (All Some).
    /// Called on Save
    pub fn from_resources(
        video: &VideoSettings,
        control: &ControlSettings,
        sound: &SoundSettings,
        gameplay: &GameplaySettings,
    ) -> Self {
        Self {
            version: SETTINGS_FORMAT_VERSION,
            video: VideoDto {
                vsync: Some(video.vsync),
                display_mode: Some(display_mode_to_str(video.display_mode).to_string()),
                resolution: Some([video.resolution.0, video.resolution.1]),
                fov: Some(video.fov),
                view_size: Some(video.view_size),
                msaa: Some(msaa_to_str(video.msaa).to_string()),
                render_scale: Some(render_scale_to_str(video.render_scale).to_string()),
            },
            control: ControlDto {
                mouse_sensitivity: Some(control.mouse_sensitivity),
                invert_y: Some(control.invert_y),
                mouselook_enabled: Some(control.mouselook_enabled),
                gamepad_enabled: Some(control.gamepad_enabled),
                gamepad_sensitivity: Some(control.gamepad_sensitivity),
                gamepad_deadzone: Some(control.gamepad_deadzone),
            },
            sound: SoundDto {
                master_volume: Some(sound.master_volume),
                music_volume: Some(sound.music_volume),
                sfx_volume: Some(sound.sfx_volume),
                music_enabled: Some(sound.music_enabled),
                sfx_enabled: Some(sound.sfx_enabled),
            },
            gameplay: GameplayDto {
                reversible_pushwalls: Some(gameplay.reversible_pushwalls),
            },
        }
    }

    /// Overwrite the Live Resources With the Fields Present in This DTO. Absent
    /// Fields (None) and Unrecognized Enum Strings Leave the Resource Untouched,
    /// so the Player Keeps a Sensible Default for Anything the File Does Not Carry.
    /// Called on Load
    pub fn apply(
        &self,
        video: &mut VideoSettings,
        control: &mut ControlSettings,
        sound: &mut SoundSettings,
        gameplay: &mut GameplaySettings,
    ) {
        // --- Video ---
        if let Some(v) = self.video.vsync {
            video.vsync = v;
        }
        if let Some(s) = &self.video.display_mode {
            if let Some(m) = display_mode_from_str(s) {
                video.display_mode = m;
            }
        }
        if let Some(r) = self.video.resolution {
            video.resolution = (r[0], r[1]);
        }
        if let Some(f) = self.video.fov {
            video.fov = f;
        }
        if let Some(vs) = self.video.view_size {
            video.view_size = vs;
        }
        if let Some(s) = &self.video.msaa {
            if let Some(m) = msaa_from_str(s) {
                video.msaa = m;
            }
        }
        if let Some(s) = &self.video.render_scale {
            if let Some(r) = render_scale_from_str(s) {
                video.render_scale = r;
            }
        }

        // --- Control (Key Bindings Excluded for Now) ---
        if let Some(v) = self.control.mouse_sensitivity {
            control.mouse_sensitivity = v;
        }
        if let Some(v) = self.control.invert_y {
            control.invert_y = v;
        }
        if let Some(v) = self.control.mouselook_enabled {
            control.mouselook_enabled = v;
        }
        if let Some(v) = self.control.gamepad_enabled {
            control.gamepad_enabled = v;
        }
        if let Some(v) = self.control.gamepad_sensitivity {
            control.gamepad_sensitivity = v;
        }
        if let Some(v) = self.control.gamepad_deadzone {
            control.gamepad_deadzone = v;
        }

        // --- Sound ---
        if let Some(v) = self.sound.master_volume {
            sound.master_volume = v;
        }
        if let Some(v) = self.sound.music_volume {
            sound.music_volume = v;
        }
        if let Some(v) = self.sound.sfx_volume {
            sound.sfx_volume = v;
        }
        if let Some(v) = self.sound.music_enabled {
            sound.music_enabled = v;
        }
        if let Some(v) = self.sound.sfx_enabled {
            sound.sfx_enabled = v;
        }

        // --- Gameplay ---
        if let Some(v) = self.gameplay.reversible_pushwalls {
            gameplay.reversible_pushwalls = v;
        }
    }
}
