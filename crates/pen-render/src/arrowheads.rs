use crate::{DrawList, Prim};
use pen_core::{Path, Props};

pub(crate) fn draw(path: &Path, props: &Props, out: &mut DrawList) {
    let mut color = props.stroke;
    color.a = (u16::from(color.a) * u16::from(props.opacity.min(100)) / 100) as u8;
    for head in pen_core::arrowheads(path, props) {
        if head.filled {
            out.push(Prim::Fill {
                points: head.points,
                color,
            });
        } else {
            out.push(Prim::Stroke {
                points: head.points,
                closed: head.closed,
                width: props.stroke_width,
                color,
            });
        }
    }
}
