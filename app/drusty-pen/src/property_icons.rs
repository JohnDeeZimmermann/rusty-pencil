//! Small outline icons in the same 20-point grid as the drawing toolbar.
use crate::design as d;
use pen_core::{Font, Roughness, Srgba, TextAlign};
use rune::Shape;

fn stroke(shape: Shape, points: &[[f32; 2]], color: Srgba) -> Shape {
    shape.stroke(points.to_vec(), d::ICON_STROKE, d::c(color))
}

pub(crate) fn font(font: Font, color: Srgba) -> Shape {
    match font {
        Font::HandDrawn => {
            let shape = stroke(
                Shape::new(),
                &[
                    [4.0, 16.0],
                    [5.0, 12.0],
                    [14.0, 3.0],
                    [17.0, 6.0],
                    [8.0, 15.0],
                    [4.0, 16.0],
                ],
                color,
            );
            let shape = stroke(shape, &[[12.0, 5.0], [15.0, 8.0]], color);
            stroke(shape, &[[3.0, 18.0], [10.0, 18.0]], color)
        }
        Font::Normal => {
            let shape = stroke(
                Shape::new(),
                &[[3.0, 17.0], [9.0, 3.0], [11.0, 3.0], [17.0, 17.0]],
                color,
            );
            stroke(shape, &[[6.0, 12.0], [14.0, 12.0]], color)
        }
        Font::Code => {
            let shape = stroke(Shape::new(), &[[6.0, 5.0], [2.0, 10.0], [6.0, 15.0]], color);
            let shape = stroke(shape, &[[14.0, 5.0], [18.0, 10.0], [14.0, 15.0]], color);
            stroke(shape, &[[12.0, 3.0], [8.0, 17.0]], color)
        }
    }
}

pub(crate) fn align(align: TextAlign, color: Srgba) -> Shape {
    (0..4).fold(Shape::new(), |shape, i| {
        let width = if i % 2 == 0 { 16.0 } else { 10.0 };
        let x = match align {
            TextAlign::Left => 2.0,
            TextAlign::Centre => (d::ICON - width) / 2.0,
            TextAlign::Right => 18.0 - width,
        };
        let y = 4.0 + i as f32 * 4.0;
        stroke(shape, &[[x, y], [x + width, y]], color)
    })
}

pub(crate) fn roughness(roughness: Roughness) -> Shape {
    let points: &[[f32; 2]] = match roughness {
        Roughness::Architect => &[[2.0, 14.0], [18.0, 6.0]],
        Roughness::Artist => &[
            [2.0, 14.0],
            [6.0, 11.0],
            [10.0, 10.0],
            [14.0, 7.0],
            [18.0, 6.0],
        ],
        Roughness::Cartoonist => &[
            [2.0, 14.0],
            [5.0, 9.0],
            [8.0, 13.0],
            [12.0, 6.0],
            [15.0, 9.0],
            [18.0, 5.0],
        ],
    };
    stroke(Shape::new(), points, d::INK)
}
