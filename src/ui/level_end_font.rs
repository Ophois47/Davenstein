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

fn glyph_rect_and_advance(c: char) -> (Rect, f32) {
    // Keep these local so we don't depend on any other constants.
    const GLYPH: u32 = 16;
    const SEP: u32 = 1;

    // Tiny epsilon so we don't ever land exactly on a neighbor edge.
    // IMPORTANT: this is *not* an inset big enough to cut strokes—just avoids edge sampling.
    const EPS: f32 = 0.01;

    // Full-cell rect (16px wide), starting at col*(16+1), row*(16+1)
    let rect_full = |col: u32, row: u32| {
        let x0 = col * (GLYPH + SEP);
        let y0 = row * (GLYPH + SEP);
        Rect::from_corners(
            Vec2::new(x0 as f32 + EPS, y0 as f32 + EPS),
            Vec2::new((x0 + GLYPH) as f32 - EPS, (y0 + GLYPH) as f32 - EPS),
        )
    };

    // Sub-rect inside a cell (for shared-cell punctuation)
    let rect_sub = |col: u32, row: u32, x_off: u32, w: u32| {
        let x0 = col * (GLYPH + SEP) + x_off;
        let y0 = row * (GLYPH + SEP);
        Rect::from_corners(
            Vec2::new(x0 as f32 + EPS, y0 as f32 + EPS),
            Vec2::new((x0 + w) as f32 - EPS, (y0 + GLYPH) as f32 - EPS),
        )
    };

    let one = 1.0;

    match c.to_ascii_uppercase() {
        // digits
        '0' => (rect_full(0, 0), one),
        '1' => (rect_full(1, 0), one),
        '2' => (rect_full(2, 0), one),
        '3' => (rect_full(3, 0), one),
        '4' => (rect_full(4, 0), one),
        '5' => (rect_full(5, 0), one),
        '6' => (rect_full(6, 0), one),
        '7' => (rect_full(7, 0), one),
        '8' => (rect_full(8, 0), one),
        '9' => (rect_full(9, 0), one),

        // A..J
        'A' => (rect_full(0, 1), one),
        'B' => (rect_full(1, 1), one),
        'C' => (rect_full(2, 1), one),
        'D' => (rect_full(3, 1), one),
        'E' => (rect_full(4, 1), one),
        'F' => (rect_full(5, 1), one),
        'G' => (rect_full(6, 1), one),
        'H' => (rect_full(7, 1), one),
        'I' => (rect_full(8, 1), one),
        'J' => (rect_full(9, 1), one),

        // K..T
        'K' => (rect_full(0, 2), one),
        'L' => (rect_full(1, 2), one),
        'M' => (rect_full(2, 2), one),
        'N' => (rect_full(3, 2), one),
        'O' => (rect_full(4, 2), one),
        'P' => (rect_full(5, 2), one),
        'Q' => (rect_full(6, 2), one),
        'R' => (rect_full(7, 2), one),
        'S' => (rect_full(8, 2), one),
        'T' => (rect_full(9, 2), one),

        // U..Z
        'U' => (rect_full(0, 3), one),
        'V' => (rect_full(1, 3), one),
        'W' => (rect_full(2, 3), one),
        'X' => (rect_full(3, 3), one),
        'Y' => (rect_full(4, 3), one),
        'Z' => (rect_full(5, 3), one),

        // punctuation
        ':' => (rect_full(6, 3), one),

        // Row 3 col 7 is shared: [% .... !]
        // Split point that matches your sheet: '!' starts at x=11 and is 5px wide (11..15).
        '%' => {
            let w = 11; // 0..10 is the percent region (includes its dot at x=8)
            (rect_sub(7, 3, 0, w), w as f32 / GLYPH as f32)
        }
        '!' => {
            let x_off = 11;
            let w = 5;
            (rect_sub(7, 3, x_off, w), w as f32 / GLYPH as f32)
        }

        // Apostrophe is in row 3 col 8 but that cell contains white area.
        // Only sample the left teal part (skip the fully-white column 0 and the white right half).
        '\'' => {
            let x_off = 1;
            let w = 8; // columns 1..8 are the teal region; avoids the white right side
            (rect_sub(8, 3, x_off, w), w as f32 / GLYPH as f32)
        }

        // Keep '?' as a safe fallback; your sheet’s (3,9) is blank, so this effectively shows nothing.
        '?' => (rect_full(9, 3), one),

        _ => (rect_full(0, 0), one),
    }
}

fn hud_scale_i(q_windows: &Query<&Window, With<PrimaryWindow>>) -> f32 {
    const BASE_W: f32 = 320.0;

    let Some(win) = q_windows.iter().next() else { return 1.0; };
    (win.resolution.width() / BASE_W).floor().max(1.0)
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

    for (e, bt, kids) in q_text.iter() {
        let glyph_px = 16.0 * base_scale * bt.scale;

        // Clear Old Glyphs
        if let Some(kids) = kids {
            for k in kids.iter() {
                commands.entity(k).despawn();
            }
        }

        // Rebuild Glyphs
        commands.entity(e).with_children(|ui| {
            for ch in bt.text.chars() {
                if ch == ' ' {
                    ui.spawn(Node {
                        width: Val::Px(glyph_px),
                        height: Val::Px(glyph_px),
                        ..default()
                    });
                    continue;
                }

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
        });
    }
}
