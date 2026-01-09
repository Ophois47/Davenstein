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

fn glyph_cell(c: char) -> (usize, usize) {
    let c = c.to_ascii_uppercase();

    match c {
        // Digits
        '0'..='9' => (0, (c as u8 - b'0') as usize),

        // Letters A..J
        'A'..='J' => (1, (c as u8 - b'A') as usize),

        // Letters K..T
        'K'..='T' => (2, (c as u8 - b'K') as usize),

        // Letters U..Z
        'U'..='Z' => (3, (c as u8 - b'U') as usize),

        // Punctuation row (row 3, cols 6..9 in your sheet)
        // NOTE: '%' and '!' share col 7 in your current atlas; ':' is col 6.
        ':' => (3, 6),
        '%' => (3, 7),
        '!' => (3, 7),
        '\'' => (3, 9),

        // Space -> treat as a blank cell (you can also special-case to "advance only")
        ' ' => (0, 0),

        // Fallback: show '0' for unknown characters
        _ => (0, 0),
    }
}

fn glyph_rect_and_advance(c: char) -> (Rect, f32) {
    // Your sheet is: 16px glyphs, 1px separators between cells.
    const GLYPH: u32 = 16;
    const SEP: u32 = 1;
    const STRIDE: u32 = GLYPH + SEP;

    // Tiny inset so we don’t sample neighbor pixels at edges (prevents bleed).
    const EPS: f32 = 0.01;

    let rect_full = |col: u32, row: u32| {
        let x0 = col * STRIDE;
        let y0 = row * STRIDE;
        Rect::from_corners(
            Vec2::new(x0 as f32 + EPS, y0 as f32 + EPS),
            Vec2::new((x0 + GLYPH) as f32 - EPS, (y0 + GLYPH) as f32 - EPS),
        )
    };

    let rect_sub = |col: u32, row: u32, x_off: u32, w: u32| {
        let x0 = col * STRIDE + x_off;
        let y0 = row * STRIDE;
        Rect::from_corners(
            Vec2::new(x0 as f32 + EPS, y0 as f32 + EPS),
            Vec2::new((x0 + w) as f32 - EPS, (y0 + GLYPH) as f32 - EPS),
        )
    };

    let c = c.to_ascii_uppercase();

    // Default: alphanumerics map through glyph_cell (row,col)
    let default_full = || {
        let (row, col) = glyph_cell(c);
        (rect_full(col as u32, row as u32), 1.0)
    };

    match c {
        // Special punctuation row (row 3)
        ':' => (rect_full(6, 3), 1.0),

        // IMPORTANT: row 3 col 7 contains BOTH '%' and '!' with a white divider at local x=8.
        // '%' is on the LEFT. We also include the 1px *left* separator column because the art bleeds into it.
        // That left separator is at global x = (7*STRIDE - 1). We take 9px total: [-1 .. 7], and EXCLUDE local x=8 (white).
        '%' => {
            let col = 7u32;
            let row = 3u32;
            let x0 = col * STRIDE;
            let y0 = row * STRIDE;

            // Start one pixel left of the cell, end before local x=8 (the internal white divider).
            // Range: (x0 - 1) .. (x0 + 8)  => width 9px
            let rect = Rect::from_corners(
                Vec2::new((x0 - 1) as f32 + EPS, y0 as f32 + EPS),
                Vec2::new((x0 + 8) as f32 - EPS, (y0 + GLYPH) as f32 - EPS),
            );

            // Advance matches sampled width so the glyph isn't stretched.
            (rect, 9.0 / 16.0)
        }

        // '!' is on the RIGHT side of that same shared cell: local x = 11..15 (5px wide).
        '!' => (rect_sub(7, 3, 11, 5), 5.0 / 16.0),

        // Apostrophe lives in row 3 col 8, but that cell is half white.
        // Only use the left teal region: local x = 1..8 (8px wide). (col0 is white)
        '\'' => (rect_sub(8, 3, 1, 8), 8.0 / 16.0),

        _ => default_full(),
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
