//! Native context menus, with a token-styled fallback on platforms without Rune menus.
use crate::{State, design as d};
use pen_editor::{Action, InputEvent, ScreenPt};
use rune::{El, PointerEvent, PopupItem, PopupMenu, Tx, col, el, row, text};

pub(crate) fn open(s: &mut State, press: PointerEvent, tx: &mut Tx) {
    let at = ScreenPt::new(press.x, press.y);
    s.input(InputEvent::Context { at }, tx);
    if rune::popup_menu_supported() {
        let menu = Action::ALL.into_iter().fold(PopupMenu::new(), |menu, a| {
            let mut item = PopupItem::new(a.id(), a.label()).enabled(a.enabled(&s.editor));
            if let Ok(key) = a.shortcut().replace("Ctrl", "cmd").to_lowercase().parse() {
                item = item.key_equivalent(key);
            }
            menu.item(item)
        });
        tx.popup_menu(menu, (press.x, press.y), |s: &mut State, picked, tx| {
            if let Some(a) = Action::ALL.into_iter().find(|a| Some(a.id()) == picked) {
                s.run_action(a, tx);
            }
        });
    } else {
        s.context = Some(at);
    }
}

pub(crate) fn menu(s: &State) -> El {
    let Some(at) = s.context else {
        return el().size(0.0, 0.0);
    };
    let dark = s.editor.settings().dark;
    let c = |v| d::themed(v, dark);
    let viewport = s.editor.viewport();
    let x =
        at.x.min((viewport.dx - d::MENU_WIDTH - d::S2).max(d::S2))
            .max(d::S2);
    let y =
        at.y.min((viewport.dy - d::MENU_HEIGHT - d::S2).max(d::S2))
            .max(d::S2);
    let column = |actions: &[Action]| {
        actions.iter().fold(col().gap(2.0), |node, &a| {
            let enabled = a.enabled(&s.editor);
            node.child(
                row()
                    .key(format!("action-{}", a.id()))
                    .size(d::MENU_COLUMN, d::MENU_ROW)
                    .px(d::S2)
                    .center_y()
                    .justify_between()
                    .radius(d::R_HANDLE)
                    .hover(if enabled {
                        c(d::ACCENT_SOFT)
                    } else {
                        rune::Color::TRANSPARENT
                    })
                    .child(text(a.label()).font(d::T_CONTROL).color(c(if enabled {
                        d::INK
                    } else {
                        d::MUTED
                    })))
                    .child(text(a.shortcut()).font(d::T_BADGE).color(c(d::MUTED)))
                    .on_press_tx(move |s: &mut State, tx| {
                        s.run_action(a, tx);
                    }),
            )
        })
    };
    el().key("context-dismiss")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .on_press_tx(|s: &mut State, _| s.context = None)
        .child(
            d::surface(row().key("context-menu"), d::R_ISLAND, dark)
                .absolute(x, y)
                .padding(d::S2)
                .gap(4.0)
                .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
                .child(column(&Action::ALL[..13]))
                .child(
                    el().size(d::OUTLINE, d::MENU_HEIGHT - 28.0)
                        .fill(c(d::BORDER)),
                )
                .child(column(&Action::ALL[13..])),
        )
}

#[cfg(test)]
mod tests {
    use crate::tests::{FRAME, app, drag};

    #[test]
    fn secondary_click_and_fallback_action_use_the_registry_without_drawing() {
        let mut rt = app();
        drag(&mut rt, (310.0, 250.0), (480.0, 390.0));
        rt.secondary_down(310.0, 280.0);
        rt.secondary_up(310.0, 280.0);
        rt.frame(FRAME);
        if !rune::popup_menu_supported() {
            let (x, y) = rt.find("action-duplicate").expect("fallback menu");
            rt.pointer_move(x, y);
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(FRAME);
            assert_eq!(rt.state.editor.document().elements().len(), 2);
            assert!(rt.state.context.is_none());
            assert!(rt.find("context-menu").is_none());
        }
    }

    #[test]
    fn clicking_outside_the_menu_dismisses_without_starting_a_canvas_gesture() {
        let mut rt = app();
        rt.secondary_down(350.0, 300.0);
        rt.secondary_up(350.0, 300.0);
        rt.frame(FRAME);
        if !rune::popup_menu_supported() {
            drag(&mut rt, (900.0, 300.0), (950.0, 350.0));
            assert!(rt.state.context.is_none());
            assert!(rt.state.editor.document().elements().is_empty());
        }
    }
    #[test]
    fn copy_closes_the_fallback_even_when_no_document_paint_changes() {
        let mut rt = app();
        let clipboard = rune::clipboard::memory();
        rt.use_clipboard(&clipboard);
        drag(&mut rt, (310.0, 250.0), (480.0, 390.0));
        rt.secondary_down(310.0, 280.0);
        rt.secondary_up(310.0, 280.0);
        rt.frame(FRAME);
        if !rune::popup_menu_supported() {
            let (x, y) = rt.find("action-copy").unwrap();
            rt.pointer_move(x, y);
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(FRAME);
            assert!(rt.find("context-menu").is_none());
            assert!(clipboard.content().is_some());
            assert_eq!(rt.state.editor.document().elements().len(), 1);
        }
    }
}
