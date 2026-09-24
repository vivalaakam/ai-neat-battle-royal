use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use bevy::prelude::*;
use bevy::text::FontCx;

/// Bundled monochrome emoji font (outline glyphs — works with Bevy/swash).
/// Color CBDT fonts like NotoColorEmoji panic the rasterizer.
pub const EMOJI_FONT_ASSET: &str = "fonts/NotoEmoji.ttf";
const EMOJI_FONT_FAMILY: &str = "Noto Emoji";
const EMOJI_FONT_URL: &str =
    "https://raw.githubusercontent.com/google/fonts/main/ofl/notoemoji/NotoEmoji%5Bwght%5D.ttf";

#[derive(Resource, Clone)]
pub struct EmojiFont(pub Handle<Font>);

pub fn emoji_font_path() -> PathBuf {
    Path::new("assets").join(EMOJI_FONT_ASSET)
}

/// Downloads the emoji font into `assets/` if missing. Call before `App::run`.
pub fn ensure_emoji_font() {
    let path = emoji_font_path();
    if path.is_file()
        && path
            .metadata()
            .map(|meta| meta.len() > 10_000)
            .unwrap_or(false)
    {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    eprintln!("emoji font missing — downloading Noto Emoji → {}", path.display());
    let dest = path.to_string_lossy().into_owned();
    let ok = download_with("curl", &["-fsSL", EMOJI_FONT_URL, "-o", &dest])
        || download_with("wget", &["-q", EMOJI_FONT_URL, "-O", &dest]);
    if !ok || !path.is_file() {
        eprintln!(
            "warning: failed to download emoji font; leaderboard emoji may render as tofu.\n\
             place a Noto Emoji TTF at {}",
            path.display()
        );
        let _ = fs::remove_file(&path);
    } else {
        eprintln!(
            "emoji font ready ({} bytes)",
            path.metadata().map(|m| m.len()).unwrap_or(0)
        );
    }
}

fn download_with(bin: &str, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn load_emoji_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load(EMOJI_FONT_ASSET);
    commands.insert_resource(EmojiFont(handle));
}

/// Once the asset is in the font collection, map GenericFamily::Emoji → Noto Emoji
/// so missing glyphs in the default UI font fall back correctly.
pub fn configure_emoji_font(
    mut font_cx: ResMut<FontCx>,
    emoji: Option<Res<EmojiFont>>,
    fonts: Res<Assets<Font>>,
    mut configured: Local<bool>,
) {
    if *configured {
        return;
    }
    let Some(emoji) = emoji else {
        return;
    };
    if fonts.get(&emoji.0).is_none() {
        return;
    }
    match font_cx.set_emoji_family(EMOJI_FONT_FAMILY) {
        Ok(()) => {
            *configured = true;
        }
        Err(_) => {
            // Collection hasn't ingested the asset yet — retry next frame.
        }
    }
}
