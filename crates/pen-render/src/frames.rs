//! Shared clipping for retained scene parts and export.
use crate::{DrawList, Prim, draw_element};
use pen_core::{Document, Element};
pub(crate) fn append(
    doc: &Document,
    preview: &[Element],
    element: &Element,
    prims: &[Prim],
    out: &mut DrawList,
) {
    if let Some(frame) = element.frame_id.and_then(|id| {
        preview
            .iter()
            .find(|e| e.id() == id)
            .or_else(|| doc.get(id))
    }) {
        out.push(Prim::Clip {
            bounds: frame.kind.bounds(),
            prims: prims.to_vec(),
        });
    } else {
        for prim in prims {
            out.push(prim.clone());
        }
    }
}
pub fn draw(doc: &Document, preview: &[Element], element: &Element, out: &mut DrawList) {
    let mut list = DrawList::new();
    draw_element(element, &mut list);
    append(doc, preview, element, list.prims(), out);
}
