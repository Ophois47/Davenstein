/*
Davenstein - by David Petnick
*/

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Resource, Clone)]
pub(crate) struct LevelEndFont {
    pub sheet: Handle<Image>,
}

#[derive(Component, Clone)]
pub(crate) struct LevelEndBitmapText {
    pub text: String,
    pub scale: f32,
}

fn hud_scale_i(q_windows: &Query<&Window, With<PrimaryWindow>>) -> f32 {
    const BASE_W: f32 = 320.0;

    let Some(win) = q_windows.iter().next() else { return 1.0; };
    (win.resolution.width() / BASE_W).floor().max(1.0)
}

fn glyph_cell(c: char) -> (usize, usize) {
    let c = c.to_ascii_uppercase();

    // Row 0: 0..9
    if ('0'..='9').contains(&c) {
        return (0, (c as u8 - b'0') as usize);
    }

    // Row 1: A..J
    // Row 2: K..T
    // Row 3: U..Z
    if ('A'..='Z').contains(&c) {
        let idx = (c as u8 - b'A') as usize; // 0..25
        let row = 1 + (idx / 10); // 1..3
        let col = idx % 10;
        return (row, col);
    }

    // Fallback for Unknown
    (3, 0)
}

/// Half-Texel Inset Applied to the Top and Bottom Edge of Every Sampled Glyph
/// Rect, in Atlas Pixels. The Level-End Text Renders on the World/HUD Canvas
/// Camera and Is Sized in Canvas-Pixel Space, and the Intermission Overlay
/// Further Multiplies the Glyph Size by a Fractional TEXT_SCALE (0.90). A 16px
/// Glyph Cell Therefore Maps to a Non-Integer Number of Destination Pixels at
/// Many Resolutions and Video Modes. Under Linear Filtering the Sampler Then
/// Reaches Up to Half a Texel Past the Cell's Top or Bottom Edge into the 1px
/// White Separator Row Between Atlas Rows, Painting a Thin White Seam Above or
/// Below the Letters - Visible Only Where the Cell Lands on a Fractional
/// Vertical Boundary. Pulling the Top Edge Down and the Bottom Edge Up by Half
/// a Texel Snaps Both Edges Onto the First and Last Glyph Texel-Row Centers, so
/// Every Bilinear Tap Stays Inside the Glyph and the Seam Cannot Appear. It Is
/// a No-Op Under Nearest-Neighbor Sampling. Only the Vertical Edges Are Inset,
/// Because the Seam Is Top/Bottom Only, so the Carefully Chosen Horizontal
/// Divider-Avoidance Offsets for ':' '%' '!' Are Left Bit-for-Bit Unchanged
const V_INSET: f32 = 0.5;

fn glyph_rect_full(row: usize, col: usize) -> Rect {
    // 16x16 Glyphs with 1px Separators
    const GLYPH: f32 = 16.0;
    const SEP: f32 = 1.0;
    const STRIDE: f32 = GLYPH + SEP;

    let x0 = col as f32 * STRIDE;
    let y0 = row as f32 * STRIDE;

    // Inset the Top/Bottom by Half a Texel to Prevent White-Separator Bleed
    Rect::from_corners(
        Vec2::new(x0, y0 + V_INSET),
        Vec2::new(x0 + GLYPH, y0 + GLYPH - V_INSET),
    )
}

fn glyph_rect_sub(row: usize, col: usize, x_off: f32, w: f32) -> Rect {
    const GLYPH: f32 = 16.0;
    const SEP: f32 = 1.0;
    const STRIDE: f32 = GLYPH + SEP;

    let x0 = col as f32 * STRIDE + x_off;
    let y0 = row as f32 * STRIDE;

    // Inset the Top/Bottom by Half a Texel to Prevent White-Separator Bleed
    // Horizontal Extent Is Left Exactly as Requested so Divider Avoidance Holds
    Rect::from_corners(
        Vec2::new(x0, y0 + V_INSET),
        Vec2::new(x0 + w, y0 + GLYPH - V_INSET),
    )
}

fn glyph_rect_and_advance(c: char) -> (Rect, f32) {
    let c = c.to_ascii_uppercase();

    // IMPORTANT:
    // ':' '%' '!' are handled specially in sync_level_end_bitmap_text
    // because they are split/composited in your atlas.
    match c {
        // Treat '?' (and unknown) as a full-cell fallback
        _ => {
            let (row, col) = glyph_cell(c);
            (glyph_rect_full(row, col), 1.0)
        }
    }
}

pub(crate) fn sync_level_end_bitmap_text(
    mut commands: Commands,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    font: Option<Res<LevelEndFont>>,
    q_text: Query<
        (Entity, &LevelEndBitmapText, Option<&Children>),
        Or<(Added<LevelEndBitmapText>, Changed<LevelEndBitmapText>)>,
    >,
) {
    let Some(font) = font else { return; };
    let base_scale = hud_scale_i(&q_windows);

    // Atlas facts (from your sheet):
    // - Punctuation row is row=3
    // - Col 6 + Col 7 are split by an internal white divider at local x=8
    // - ':'  is left half of (3,6)      [0..8)
    // - '%'  is composite: right half of (3,6) + left half of (3,7)
    // - '!'  is right half of (3,7)
    // - Avoid sampling the divider column at local x=8 (it is solid white)

    for (e, bt, kids) in q_text.iter() {
        let glyph_px = 16.0 * base_scale * bt.scale;

        // Clear old glyphs
        if let Some(kids) = kids {
            for k in kids.iter() {
                commands.entity(k).despawn();
            }
        }

        commands.entity(e).with_children(|ui| {
            // tiny helper for spacer widths in "source pixels"
            let px = |src_px: f32| glyph_px * (src_px / 16.0);

            for ch in bt.text.chars() {
                let ch = ch.to_ascii_uppercase();

                if ch == ' ' {
                    ui.spawn(Node {
                        width: Val::Px(glyph_px),
                        height: Val::Px(glyph_px),
                        ..default()
                    });
                    continue;
                }

                match ch {
                    '\'' => {
                        // apostrophe - row 3, col 8 - sample just the glyph
                        let rect = glyph_rect_sub(3, 8, 6.0, 3.0);
                        
                        let mut img = ImageNode::new(font.sheet.clone());
                        img.rect = Some(rect);
                        
                        ui.spawn((
                            img,
                            Node {
                                width: Val::Px(px(3.0)),
                                height: Val::Px(glyph_px),
                                ..default()
                            },
                        ));
                    }

                    ':' => {
                        // left half of (3,6): [0..8)
                        let rect = glyph_rect_sub(3, 6, 0.0, 8.0);

                        let mut img = ImageNode::new(font.sheet.clone());
                        img.rect = Some(rect);

                        ui.spawn((
                            img,
                            Node {
                                width: Val::Px(px(8.0)),
                                height: Val::Px(glyph_px),
                                ..default()
                            },
                        ));

                        // Optional 1px Teal Spacing Instead of Sampling White Divider
                        ui.spawn(Node {
                            width: Val::Px(px(1.0)),
                            height: Val::Px(glyph_px),
                            ..default()
                        });
                    }

                    '%' => {
                        // Compose '%' from:
                        // - right half of (3,6): [9..16) (7px wide)
                        // - 1px spacer (teal)
                        // - left half of (3,7): [0..8)  (8px wide)

                        // Right half of col6 (skip divider at x=8)
                        {
                            let rect = glyph_rect_sub(3, 6, 9.0, 7.0);
                            let mut img = ImageNode::new(font.sheet.clone());
                            img.rect = Some(rect);

                            ui.spawn((
                                img,
                                Node {
                                    width: Val::Px(px(7.0)),
                                    height: Val::Px(glyph_px),
                                    ..default()
                                },
                            ));
                        }

                        // 1px teal spacer (replaces the divider column cleanly)
                        ui.spawn(Node {
                            width: Val::Px(px(1.0)),
                            height: Val::Px(glyph_px),
                            ..default()
                        });

                        // Left half of col7
                        {
                            let rect = glyph_rect_sub(3, 7, 0.0, 8.0);
                            let mut img = ImageNode::new(font.sheet.clone());
                            img.rect = Some(rect);

                            ui.spawn((
                                img,
                                Node {
                                    width: Val::Px(px(8.0)),
                                    height: Val::Px(glyph_px),
                                    ..default()
                                },
                            ));
                        }
                    }

                    '!' => {
                        // right half of (3,7): [9..16) (7px)
                        let rect = glyph_rect_sub(3, 7, 9.0, 7.0);

                        let mut img = ImageNode::new(font.sheet.clone());
                        img.rect = Some(rect);

                        ui.spawn((
                            img,
                            Node {
                                width: Val::Px(px(7.0)),
                                height: Val::Px(glyph_px),
                                ..default()
                            },
                        ));
                    }

                    _ => {
                        // Normal glyph path
                        let (rect, adv) = glyph_rect_and_advance(ch);
                        let w_px = glyph_px * adv;

                        let mut img = ImageNode::new(font.sheet.clone());
                        img.rect = Some(rect);

                        ui.spawn((
                            img,
                            Node {
                                width: Val::Px(w_px),
                                height: Val::Px(glyph_px),
                                ..default()
                            },
                        ));
                    }
                }
            }
        });
    }
}
