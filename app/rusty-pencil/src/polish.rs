//! Small secondary controls and searchable, keyboard-accessible local commands.
use crate::{State, design as d};
use pen_core::{Kind, Link};
use pen_editor::{Action, InputEvent, Tool};
use rune::{El, Key, KeyEvent, TextEdit, Tx, col, el, row, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Modal {
    Find,
    Commands,
    Help,
    Link,
}
#[derive(Debug, Default)]
pub(crate) struct Panel {
    pub modal: Option<Modal>,
    pub query: TextEdit,
    pub frame: TextEdit,
    pub geometry: [TextEdit; 5],
    selected: usize,
    pub link_gesture: bool,
    pub last_tick: Option<f64>,
}
pub(crate) fn button(label: &str, key: &str, active: bool, dark: bool) -> El {
    el().key(key)
        .role("button")
        .label(label)
        .h(d::CONTROL)
        .px(d::S2)
        .radius(d::R_CONTROL)
        .fill(d::themed(
            if active { d::ACCENT } else { d::ACCENT_SOFT },
            dark,
        ))
        .hover(d::themed(d::BORDER, dark))
        .items_center()
        .justify_center()
        .child(
            text(label)
                .font(d::T_CONTROL)
                .color(d::themed(d::INK, dark)),
        )
}
pub(crate) fn open(s: &mut State, modal: Modal, tx: &mut Tx) {
    let mut edit_tx = Tx::default();
    s.sync_text(&mut edit_tx);
    s.input(InputEvent::FinishText, &mut edit_tx);
    s.input(InputEvent::Cancel, &mut edit_tx);
    tx.wake_after(std::time::Duration::from_secs(1));
    s.menu = None;
    s.context = None;
    s.polish.modal = Some(modal);
    s.polish.selected = 0;
    let value = if modal == Modal::Link {
        s.editor
            .selection()
            .ids()
            .next()
            .and_then(|id| s.editor.document().get(id))
            .and_then(|e| e.link.as_ref())
            .map(|link| match link {
                Link::Web(url) => url.clone(),
                Link::Element(id) => format!("#{}", id.get()),
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    s.polish.query = TextEdit::new(value);
    s.polish.query.focused = true;
}
pub(crate) fn key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.key == Key::Escape && s.polish.modal.take().is_some() {
        tx.blur();
        return true;
    }
    let c = match &ev.key {
        Key::Command(c) => *c,
        Key::Text(t) => t.chars().next().unwrap_or('\0'),
        _ => '\0',
    };
    let modal = if ev.mods.cmd && !ev.mods.alt {
        match c {
            'f' => Some(Modal::Find),
            '/' => Some(Modal::Commands),
            'k' => Some(Modal::Link),
            _ => None,
        }
    } else if c == '?' {
        Some(Modal::Help)
    } else {
        None
    };
    if let Some(modal) = modal {
        if modal != Modal::Link || (!s.editor.modes().view && !s.editor.selection().is_empty()) {
            open(s, modal, tx);
        }
        return true;
    }
    false
}
pub(crate) fn controls(s: &State) -> El {
    let dark = s.editor.settings().dark;
    if s.editor.modes().zen {
        return button("Exit zen · Alt+Z", "exit-zen", false, dark)
            .pin(None, Some(d::S4), Some(d::S4), None)
            .on_press_tx(|s: &mut State, tx| {
                s.input(
                    InputEvent::Key {
                        key: pen_editor::Key::Char('z'),
                        mods: pen_editor::Modifiers {
                            alt: true,
                            ..Default::default()
                        },
                    },
                    tx,
                );
            });
    }
    let mut tools = row().gap(d::S1);
    for tool in [Tool::Frame, Tool::Lasso, Tool::Laser, Tool::Eyedropper] {
        tools = tools.child(
            button(
                &tool.letter().to_ascii_uppercase().to_string(),
                &format!("extra-tool-{}", tool.letter()),
                s.editor.tool() == tool,
                dark,
            )
            .w(d::CONTROL)
            .help(format!(
                "{} · {}",
                tool.name(),
                tool.letter().to_ascii_uppercase()
            ))
            .on_press_tx(move |s: &mut State, tx| {
                s.input(InputEvent::SelectTool(tool), tx);
            }),
        );
    }
    let mut controls = col()
        .gap(d::S2)
        .child(tools)
        .child(
            row()
                .gap(d::S1)
                .child(
                    button("Library", "library-toggle", s.library.open, dark)
                        .on_press(|s: &mut State| s.library.open = !s.library.open),
                )
                .child(
                    button("", "find-open", false, dark)
                        .label("Find text")
                        .child(
                            rune::canvas(crate::property_icons::find(if dark {
                                d::INK.dark()
                            } else {
                                d::INK
                            }))
                            .size(d::ICON, d::ICON),
                        )
                        .help("Find text · Ctrl+F")
                        .on_press_tx(|s: &mut State, tx| open(s, Modal::Find, tx)),
                )
                .child(
                    button("?", "help-open", false, dark)
                        .help("Keyboard shortcuts · ?")
                        .on_press_tx(|s: &mut State, tx| open(s, Modal::Help, tx)),
                ),
        )
        .child(
            button("Commands · Ctrl+/", "commands-open", false, dark)
                .on_press_tx(|s: &mut State, tx| open(s, Modal::Commands, tx)),
        );
    if s.editor.modes().view {
        controls = controls.child(
            button("Exit view · Alt+R", "exit-view", true, dark).on_press_tx(
                |s: &mut State, tx| {
                    s.input(
                        InputEvent::Key {
                            key: pen_editor::Key::Char('r'),
                            mods: pen_editor::Modifiers {
                                alt: true,
                                ..Default::default()
                            },
                        },
                        tx,
                    );
                },
            ),
        );
    }
    d::surface(controls, d::R_ISLAND, dark)
        .key("utilities")
        .padding(d::S2)
        .pin(None, Some(d::INSPECTOR_TOP), Some(d::S4), None)
        .on_drag(|_: &mut State, _, tx| tx.live_only())
}
fn results(s: &State) -> Vec<(String, ResultEntry)> {
    let query = s.polish.query.text.to_lowercase();
    match s.polish.modal {
        Some(Modal::Find) => s
            .editor
            .document()
            .elements()
            .iter()
            .filter_map(|e| {
                let label = match &e.kind {
                    Kind::Text { content, .. } => content,
                    Kind::Frame { name, .. } => name,
                    _ => return None,
                };
                label
                    .to_lowercase()
                    .contains(&query)
                    .then(|| (label.replace('\n', " "), ResultEntry::Element(e.id())))
            })
            .collect(),
        Some(Modal::Commands) => {
            let mut entries: Vec<_> = Action::ALL
                .into_iter()
                .filter(|a| a.label().to_lowercase().contains(&query))
                .map(|a| {
                    (
                        format!("{}  {}", a.label(), a.shortcut()),
                        ResultEntry::Action(a),
                    )
                })
                .collect();
            entries.extend(
                Tool::ALL
                    .into_iter()
                    .filter(|t| t.name().to_lowercase().contains(&query))
                    .map(|t| {
                        (
                            format!("{} tool  {}", t.name(), t.letter().to_ascii_uppercase()),
                            ResultEntry::Tool(t),
                        )
                    }),
            );
            for (name, c) in [
                ("Zen mode · Alt+Z", 'z'),
                ("View mode · Alt+R", 'r'),
                ("Stats · Alt+/", '/'),
                ("Snap to objects · Alt+S", 's'),
            ] {
                if name.to_lowercase().contains(&query) {
                    entries.push((name.into(), ResultEntry::Mode(c)));
                }
            }
            entries
        }
        _ => Vec::new(),
    }
}
#[derive(Clone, Copy)]
enum ResultEntry {
    Action(Action),
    Tool(Tool),
    Element(pen_core::ElementId),
    Mode(char),
}
fn run(s: &mut State, result: ResultEntry, tx: &mut Tx) {
    tx.blur();
    match result {
        ResultEntry::Action(a) => s.run_action(a, tx),
        ResultEntry::Tool(Tool::Image) => crate::images::dialog(s, tx),
        ResultEntry::Tool(t) => {
            s.input(InputEvent::SelectTool(t), tx);
        }
        ResultEntry::Element(id) => {
            s.input(InputEvent::SelectElement(id), tx);
        }
        ResultEntry::Mode(c) => {
            s.input(
                InputEvent::Key {
                    key: pen_editor::Key::Char(c),
                    mods: pen_editor::Modifiers {
                        alt: true,
                        ..Default::default()
                    },
                },
                tx,
            );
        }
    }
    s.polish.modal = None;
}
fn apply_link(s: &mut State, tx: &mut Tx) {
    let value = s.polish.query.text.trim();
    let link = if value.is_empty() {
        None
    } else if let Some(id) = value
        .strip_prefix('#')
        .and_then(|id| id.parse::<u64>().ok())
    {
        let Some(target) = s
            .editor
            .document()
            .elements()
            .iter()
            .find(|e| e.id().get() == id)
        else {
            s.files.status.set("No element with that id");
            return;
        };
        Some(Link::Element(target.id()))
    } else if (value.starts_with("https://") || value.starts_with("http://"))
        && !value.chars().any(char::is_whitespace)
    {
        Some(Link::Web(value.into()))
    } else {
        s.files.status.set("Use an https:// URL or #element-id");
        return;
    };
    s.input(InputEvent::SetLink(link), tx);
    s.polish.modal = None;
    tx.blur();
}
pub(crate) fn dialog(s: &State) -> El {
    let Some(modal) = s.polish.modal else {
        return el().size(0.0, 0.0);
    };
    let dark = s.editor.settings().dark;
    let title = match modal {
        Modal::Find => "Find on canvas",
        Modal::Commands => "Commands",
        Modal::Help => "Keyboard shortcuts",
        Modal::Link => "Link selected elements",
    };
    let mut content = col().gap(d::S3).child(
        row()
            .justify_between()
            .child(text(title).font(d::T_LABEL).color(d::themed(d::INK, dark)))
            .child(
                button("Esc", "close-polish", false, dark).on_press_tx(|s: &mut State, tx| {
                    s.polish.modal = None;
                    tx.blur();
                }),
            ),
    );
    if modal == Modal::Help {
        let keys = crate::platform::TAB_KEYS;
        let tabs = format!(
            "{} New tab · {} Close tab · {} Next tab",
            keys.new, keys.close, keys.next
        );
        for label in [
            "V Select · R Rectangle · D Diamond · O Ellipse",
            "A Arrow · L Line · P Pen · T Text · 9 Image",
            "F Frame · S Lasso · K Laser · I Eye-dropper",
            "H / Space Pan · E Eraser · Q Lock tool",
            "Ctrl+Z Undo · Ctrl+Shift+Z Redo",
            "Ctrl+C / X / V Copy / Cut / Paste · Ctrl+D Duplicate",
            "Ctrl+G Group · Ctrl+Shift+G Ungroup",
            "Ctrl+K Link · Ctrl+F Find · Ctrl+/ Commands",
            "Alt+S Snap · Alt+Z Zen · Alt+R View · Alt+/ Stats",
            "Shift+1 Fit drawing · Shift+2 Fit selection",
            "Ctrl+S Save · Ctrl+O Open · Ctrl+Shift+E Export",
            &tabs,
        ] {
            content = content.child(
                text(label)
                    .font(d::T_CONTROL)
                    .color(d::themed(d::MUTED, dark)),
            );
        }
    } else {
        let field = rune_kit::TextInput::new(&s.polish.query)
            .plain()
            .width(None)
            .height(d::CONTROL)
            .text_style(d::T_CONTROL)
            .placeholder(if modal == Modal::Link {
                "https://… or #element-id"
            } else {
                "Type to search…"
            })
            .on_edit(|s: &mut State| &mut s.polish.query)
            .on_change(|s: &mut State, _| s.polish.selected = 0)
            .on_submit(|s: &mut State, tx| {
                if s.polish.modal == Some(Modal::Link) {
                    apply_link(s, tx);
                } else if let Some((_, entry)) = results(s).get(s.polish.selected).cloned() {
                    run(s, entry, tx);
                }
            })
            .on_key(|s: &mut State, ev, tx| {
                if ev.key == Key::Escape {
                    s.polish.modal = None;
                    tx.blur();
                    return true;
                }
                if matches!(ev.key, Key::Down | Key::Up) {
                    let count = results(s).len();
                    if count > 0 {
                        s.polish.selected = if ev.key == Key::Down {
                            (s.polish.selected + 1) % count
                        } else {
                            (s.polish.selected + count - 1) % count
                        };
                    }
                    tx.scroll_into_view(format!("result-{}", s.polish.selected));
                    return true;
                }
                false
            })
            .build(&d::kit(dark))
            .key("polish-query");
        content = content.child(field);
        if modal == Modal::Link {
            content = content
                .child(
                    text("Empty removes the link. Find element ids in Stats.")
                        .font(d::T_HINT)
                        .color(d::themed(d::MUTED, dark)),
                )
                .child(button("Apply link", "apply-link", true, dark).on_press_tx(apply_link));
        } else {
            let entries = results(s);
            let mut list = col().gap(d::S1);
            if entries.is_empty() {
                list = list.child(
                    text("No matches")
                        .font(d::T_HINT)
                        .color(d::themed(d::MUTED, dark)),
                );
            }
            for (i, (label, entry)) in entries.into_iter().enumerate() {
                let enabled = !s.editor.modes().view
                    || matches!(entry, ResultEntry::Element(_) | ResultEntry::Mode(_));
                let enabled = enabled
                    && match entry {
                        ResultEntry::Action(a) => a.enabled(&s.editor),
                        _ => true,
                    };
                list = list.child(
                    button(&label, &format!("result-{i}"), i == s.polish.selected, dark)
                        .opacity(if enabled { 1.0 } else { 0.4 })
                        .on_press_tx(move |s: &mut State, tx| {
                            if enabled {
                                run(s, entry, tx);
                            }
                        }),
                );
            }
            content = content.child(list.max_h(d::PALETTE_HEIGHT).scroll(rune::ScrollSpec {
                stick_bottom: false,
            }));
        }
    }
    el().key("polish-modal")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .fill(d::themed(d::SELECTION_WASH, dark))
        .on_drag(|_: &mut State, _, tx| tx.live_only())
        .child(
            d::surface(content, d::R_ISLAND, dark)
                .w(d::PALETTE_WIDTH)
                .padding(d::S4)
                .modal(false, |s: &mut State| s.polish.modal = None),
        )
}

pub(crate) fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    let dt = s
        .polish
        .last_tick
        .map_or(0.0, |old| (now - old).max(0.0) as f32);
    s.polish.last_tick = Some(now);
    let mut changed = false;
    if !s.editor.laser().is_empty() {
        changed = s.input(InputEvent::Tick(dt), tx) != pen_editor::Redraw::Nothing;
        tx.wake_after(std::time::Duration::from_millis(16));
    }
    crate::files::poll(s, now, tx) || changed
}
