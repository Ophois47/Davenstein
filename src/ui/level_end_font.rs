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

    // Row 3: U..Z : % ! ?
    match c {
        ':' => (3, 6),
        '%' => (3, 7),
        '!' => (3, 8),
        '?' => (3, 9),
        _ => (3, 9), // unknown => ?
    }
}

fn glyph_rect(row: usize, col: usize) -> Rect {
    // 16x16 glyphs with 1px separators
    const GLYPH: f32 = 16.0;
    const SEP: f32 = 1.0;
    const STRIDE: f32 = GLYPH + SEP;

    let x0 = col as f32 * STRIDE;
    let y0 = row as f32 * STRIDE;

    Rect::from_corners(Vec2::new(x0, y0), Vec2::new(x0 + GLYPH, y0 + GLYPH))
}

fn hud_scale_i(q_windows: &Query<&Window, With<PrimaryWindow>>) -> f32 {
    // Match your other UI integer scaling behavior
    const BASE_W: f32 = 320.0;

    let Some(win) = q_windows.iter().next() else { return 1.0; };
    (win.resolution.width() / BASE_W).floor().max(1.0)
}

pub(crate) fn sync_level_end_bitmap_text(
    mut commands: Commands,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    font: Option<Res<LevelEndFont>>,
    q_text: Query<(Entity, &LevelEndBitmapText, Option<&Children>), Or<(Added<LevelEndBitmapText>, Changed<LevelEndBitmapText>)>>,
) {
    let Some(font) = font else { return; };

    let scale = hud_scale_i(&q_windows);
    let glyph_px = 6.0 * scale;

    for (e, bt, kids) in q_text.iter() {
        // Clear old glyphs
        if let Some(kids) = kids {
            for k in kids.iter() {
                commands.entity(k).despawn();
            }
        }

        // Rebuild glyphs
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

                let (row, col) = glyph_cell(ch);
                let rect = glyph_rect(row, col);

                let mut img = ImageNode::new(font.sheet.clone());
                img.rect = Some(rect);

                ui.spawn((
                    img,
                    Node {
                        width: Val::Px(glyph_px),
                        height: Val::Px(glyph_px),
                        ..default()
                    },
                ));
            }
        });
    }
}
