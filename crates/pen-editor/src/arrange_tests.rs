use super::*;
use crate::selection_tests::{
    ALT, CTRL, NONE, SHIFT, click, down, drag, key, movement, one, p, up,
};
use pen_core::{GroupId, Path, WorldPt};

fn three() -> Editor {
    let mut ed = Editor::default();
    ed.settings.tool_lock = true;
    for (a, b) in [
        (p(100.0, 100.0), p(180.0, 160.0)),
        (p(240.0, 200.0), p(340.0, 280.0)),
        (p(420.0, 140.0), p(480.0, 190.0)),
    ] {
        drag(&mut ed, a, b, NONE);
    }
    key(&mut ed, Key::Char('v'), NONE);
    key(&mut ed, Key::Char('a'), CTRL);
    ed
}

fn contents(ed: &Editor) -> Vec<Element> {
    ed.document.elements().to_vec()
}
fn assert_content(actual: &[Element], expected: &[Element]) {
    assert_eq!(actual.len(), expected.len());
    for (a, b) in actual.iter().zip(expected) {
        assert_eq!(a.id(), b.id());
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.props, b.props);
        assert_eq!(a.angle.to_bits(), b.angle.to_bits());
        assert_eq!(a.group_ids, b.group_ids);
        assert_eq!(a.locked, b.locked);
        assert_eq!(a.z_key, b.z_key);
    }
}

#[test]
fn every_mutating_registry_action_is_one_undo_and_redo() {
    for action in Action::ALL {
        if matches!(action, Action::Copy | Action::CopyStyle) {
            continue;
        }
        let mut ed = three();
        ed.handle(InputEvent::Action(Action::Copy));
        ed.handle(InputEvent::Action(Action::CopyStyle));
        ed.handle(InputEvent::Property(pen_core::Property::Opacity(70)));
        if action == Action::Ungroup {
            ed.handle(InputEvent::Action(Action::Group));
        }
        if action == Action::UnlockAll {
            ed.handle(InputEvent::Action(Action::Lock));
        }
        if matches!(action, Action::Forward | Action::ToFront) {
            key(&mut ed, Key::Escape, NONE);
            click(&mut ed, p(140.0, 100.0), NONE);
            // Leave one element selected to make an actual order change.
        }
        if matches!(action, Action::Backward | Action::ToBack) {
            ed.selection = Selection::only(ed.document.elements()[2].id());
        }
        // Make flips visibly change asymmetric geometry, even on a single selection.
        if matches!(action, Action::FlipHorizontal | Action::FlipVertical) {
            let id = ed.document.alloc_id();
            ed.execute(
                Command::Add {
                    element: Element::new(
                        id,
                        Kind::Line(Path::from_world(&[
                            WorldPt::new(50.0, 50.0),
                            WorldPt::new(90.0, 65.0),
                            WorldPt::new(70.0, 100.0),
                        ])),
                        Props::default(),
                    ),
                    at: 3,
                },
                ed.selection.clone(),
            );
            ed.selection.insert(id);
        }
        let before = contents(&ed);
        let selection = ed.selection.clone();
        assert!(action.enabled(&ed), "{action:?}");
        assert_eq!(
            ed.handle(InputEvent::Action(action)),
            Redraw::Scene,
            "{action:?}"
        );
        let after = contents(&ed);
        key(&mut ed, Key::Char('z'), CTRL);
        assert_content(&contents(&ed), &before);
        assert_eq!(ed.selection, selection, "{action:?}");
        key(&mut ed, Key::Char('y'), CTRL);
        assert_content(&contents(&ed), &after);
    }
}

#[test]
fn rotate_live_shift_snaps_and_undo_restores_all_members() {
    let mut ed = three();
    let before = contents(&ed);
    let b = ed.selection_bounds().unwrap();
    let handle = Handle::Rotate.position(b, 1.0);
    let c = b.centre();
    down(&mut ed, p(handle.x, handle.y), NONE);
    movement(&mut ed, p(c.x + (c.y - handle.y), c.y), NONE);
    assert!(
        ed.preview()
            .iter()
            .all(|e| (e.angle - std::f32::consts::FRAC_PI_2).abs() < 0.001)
    );
    assert_content(&contents(&ed), &before);
    ed.handle(InputEvent::Modifiers(SHIFT));
    up(&mut ed, p(c.x + (c.y - handle.y), c.y), SHIFT);
    assert!(
        ed.document
            .elements()
            .iter()
            .all(|e| (e.angle - std::f32::consts::FRAC_PI_2).abs() < 0.001)
    );
    key(&mut ed, Key::Char('z'), CTRL);
    assert_content(&contents(&ed), &before);
}

#[test]
fn multi_resize_and_alt_duplicate_are_atomic_and_cancel_is_clean() {
    let mut ed = three();
    let before = contents(&ed);
    let b = ed.selection_bounds().unwrap();
    let h = Handle::Se.position(b, 1.0);
    drag(
        &mut ed,
        p(h.x, h.y),
        p(h.x + b.width(), h.y + b.height()),
        NONE,
    );
    assert!((ed.selection_bounds().unwrap().width() - 2.0 * b.width()).abs() < 0.01);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_content(&contents(&ed), &before);
    down(&mut ed, p(100.0, 110.0), ALT);
    movement(&mut ed, p(130.0, 140.0), ALT);
    assert!(
        ed.preview()
            .iter()
            .all(|e| ed.document.get(e.id()).is_none())
    );
    assert_eq!(ed.document.elements().len(), 3);
    ed.handle(InputEvent::Cancel);
    assert_content(&contents(&ed), &before);
    drag(&mut ed, p(100.0, 110.0), p(130.0, 140.0), ALT);
    assert_eq!(ed.document.elements().len(), 6);
    assert_eq!(ed.selection.len(), 3);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_content(&contents(&ed), &before);
}

#[test]
fn nested_groups_select_enter_ungroup_and_copy_with_fresh_groups() {
    let mut ed = three();
    let ids: Vec<_> = ed.selection.ids().collect();
    ed.selection = Selection::only(ids[0]);
    ed.selection.insert(ids[1]);
    key(&mut ed, Key::Char('g'), CTRL);
    ed.selection.insert(ids[2]);
    key(&mut ed, Key::Char('g'), CTRL);
    let groups = ed.document.get(ids[0]).unwrap().group_ids.clone();
    assert_eq!(groups.len(), 2);
    key(&mut ed, Key::Escape, NONE);
    click(&mut ed, p(100.0, 120.0), NONE);
    assert_eq!(ed.selection.len(), 3);
    ed.handle(InputEvent::DoubleClick {
        at: p(100.0, 120.0),
        mods: NONE,
    });
    assert_eq!(ed.selection.len(), 1);
    click(&mut ed, p(240.0, 220.0), NONE);
    assert_eq!(ed.selection.len(), 2);
    ed.handle(InputEvent::DoubleClick {
        at: p(240.0, 220.0),
        mods: NONE,
    });
    assert_eq!(ed.selection.ids().collect::<Vec<_>>(), [ids[1]]);
    key(&mut ed, Key::Escape, NONE);
    key(&mut ed, Key::Char('a'), CTRL);
    key(&mut ed, Key::Char('d'), CTRL);
    let copied: Vec<GroupId> = ed
        .selection
        .elements(&ed.document)
        .flat_map(|e| e.group_ids.clone())
        .collect();
    assert!(copied.iter().all(|g| !groups.contains(g)));
    key(
        &mut ed,
        Key::Char('g'),
        Modifiers {
            shift: true,
            ..CTRL
        },
    );
    assert!(
        ed.selection
            .elements(&ed.document)
            .all(|e| e.group_ids.len() <= 1)
    );
}

#[test]
fn lock_blocks_pointer_nudge_delete_properties_and_eraser_but_context_unlocks() {
    let mut ed = one();
    key(
        &mut ed,
        Key::Char('l'),
        Modifiers {
            shift: true,
            ..CTRL
        },
    );
    let before = contents(&ed);
    key(&mut ed, Key::Right, NONE);
    key(&mut ed, Key::Delete, NONE);
    ed.handle(InputEvent::Property(pen_core::Property::Opacity(10)));
    drag(&mut ed, p(100.0, 110.0), p(130.0, 140.0), NONE);
    key(&mut ed, Key::Char('e'), NONE);
    drag(&mut ed, p(100.0, 110.0), p(130.0, 140.0), NONE);
    assert_content(&contents(&ed), &before);
    ed.handle(InputEvent::Context {
        at: p(100.0, 150.0),
    });
    ed.handle(InputEvent::Action(Action::Lock));
    assert!(!ed.document.elements()[0].locked);
}

#[test]
fn eraser_sweeps_between_events_alt_restores_and_escape_cancels() {
    let mut ed = three();
    let before = contents(&ed);
    key(&mut ed, Key::Char('e'), NONE);
    down(&mut ed, p(40.0, 130.0), NONE);
    movement(&mut ed, p(210.0, 130.0), NONE);
    assert_eq!(ed.preview().len(), 1);
    movement(&mut ed, p(100.0, 130.0), ALT);
    assert!(ed.preview().is_empty());
    movement(&mut ed, p(180.0, 130.0), NONE);
    ed.handle(InputEvent::Cancel);
    assert_content(&contents(&ed), &before);
    drag(&mut ed, p(40.0, 130.0), p(210.0, 130.0), NONE);
    assert_eq!(ed.document.elements().len(), 2);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_content(&contents(&ed), &before);
}

#[test]
fn ordering_changes_only_moved_keys_and_alignment_uses_rotated_bounds() {
    let mut ed = three();
    let before = contents(&ed);
    ed.selection = Selection::only(before[0].id());
    ed.handle(InputEvent::Action(Action::ToFront));
    assert_eq!(ed.document.elements()[2].id(), before[0].id());
    assert_eq!(ed.document.get(before[1].id()).unwrap(), &before[1]);
    assert_eq!(ed.document.get(before[2].id()).unwrap(), &before[2]);
    ed.selection = Selection::all(&ed.document);
    ed.handle(InputEvent::Action(Action::AlignLeft));
    let left = ed.document.elements()[0].bounds().min().x;
    assert!(
        ed.document
            .elements()
            .iter()
            .all(|e| (e.bounds().min().x - left).abs() < 0.01)
    );
}

#[test]
fn all_alignment_directions_and_distributions_match_the_selection_edges() {
    for action in [
        Action::AlignLeft,
        Action::AlignCentre,
        Action::AlignRight,
        Action::AlignTop,
        Action::AlignMiddle,
        Action::AlignBottom,
    ] {
        let mut ed = three();
        ed.handle(InputEvent::Action(action));
        let coord = |e: &Element| {
            let b = e.bounds();
            match action {
                Action::AlignLeft => b.min().x,
                Action::AlignCentre => b.centre().x,
                Action::AlignRight => b.max().x,
                Action::AlignTop => b.min().y,
                Action::AlignMiddle => b.centre().y,
                _ => b.max().y,
            }
        };
        let value = coord(&ed.document.elements()[0]);
        assert!(
            ed.document
                .elements()
                .iter()
                .all(|e| (coord(e) - value).abs() < 0.001)
        );
    }
    for action in [Action::DistributeHorizontal, Action::DistributeVertical] {
        let mut ed = three();
        let horizontal = action == Action::DistributeHorizontal;
        let before = ed.selection_bounds().unwrap();
        ed.handle(InputEvent::Action(action));
        let mut bounds: Vec<_> = ed.document.elements().iter().map(Element::bounds).collect();
        bounds.sort_by(|a, b| {
            if horizontal {
                a.min().x.total_cmp(&b.min().x)
            } else {
                a.min().y.total_cmp(&b.min().y)
            }
        });
        let gaps: Vec<_> = bounds
            .windows(2)
            .map(|s| {
                if horizontal {
                    s[1].min().x - s[0].max().x
                } else {
                    s[1].min().y - s[0].max().y
                }
            })
            .collect();
        assert!((gaps[0] - gaps[1]).abs() < 0.001);
        assert_eq!(ed.selection_bounds().unwrap(), before);
    }
}

#[test]
fn duplicate_bound_text_remaps_its_owner_and_locks_follow_the_container() {
    let mut ed = one();
    let owner = ed.document.elements()[0].id();
    ed.handle(InputEvent::DoubleClick {
        at: p(150.0, 150.0),
        mods: NONE,
    });
    ed.handle(InputEvent::TextChanged {
        content: "Caption".into(),
        bounds: crate::selection_tests::r(0.0, 0.0, 70.0, 25.0),
    });
    ed.handle(InputEvent::FinishText);
    ed.selection = Selection::only(owner);
    ed.handle(InputEvent::Action(Action::Duplicate));
    let copied_owner = ed.selection.ids().next().unwrap();
    let text = pen_core::bound::bound_text(&ed.document, copied_owner).unwrap();
    assert_ne!(copied_owner, owner);
    assert_eq!(text.kind.container().unwrap().id(), copied_owner);
    ed.handle(InputEvent::Action(Action::Lock));
    assert!(
        pen_core::bound::bound_text(&ed.document, copied_owner)
            .unwrap()
            .locked
    );
    ed.handle(InputEvent::SelectTool(Tool::Select));
    ed.handle(InputEvent::DoubleClick {
        at: p(170.0, 170.0),
        mods: NONE,
    });
    assert!(ed.text_session.as_ref().is_none_or(|s| {
        s.element
            .kind
            .container()
            .is_none_or(|c| c.id() != copied_owner)
    }));
}

#[test]
fn editing_a_rotated_path_keeps_the_other_endpoint_fixed_in_world_space() {
    let mut ed = Editor::default();
    ed.handle(InputEvent::SelectTool(Tool::Line));
    drag(&mut ed, p(100.0, 100.0), p(300.0, 100.0), NONE);
    let id = ed.selection.ids().next().unwrap();
    let before = ed.document.get(id).unwrap().clone();
    let after = before.rotated(WorldPt::new(200.0, 100.0), std::f32::consts::FRAC_PI_2);
    ed.execute(Command::Patch { id, before, after }, ed.selection.clone());
    key(&mut ed, Key::Enter, CTRL);
    drag(&mut ed, p(200.0, 200.0), p(250.0, 220.0), NONE);
    let e = ed.document.get(id).unwrap();
    let path = e.kind.path().unwrap();
    let points: Vec<_> = path
        .world_points()
        .map(|p| pen_core::rotate(p, path.bounds().centre(), e.angle))
        .collect();
    assert!((points[0].x - 200.0).abs() < 0.001 && points[0].y.abs() < 0.001);
    assert!((points[1].x - 250.0).abs() < 0.001 && (points[1].y - 220.0).abs() < 0.001);
}

#[test]
fn aligning_groups_preserves_the_members_relative_positions() {
    let mut ed = three();
    let ids: Vec<_> = ed.selection.ids().collect();
    ed.selection = Selection::only(ids[0]);
    ed.selection.insert(ids[1]);
    ed.handle(InputEvent::Action(Action::Group));
    let before: Vec<_> = ed
        .selection
        .elements(&ed.document)
        .map(|e| e.bounds().centre())
        .collect();
    ed.selection.insert(ids[2]);
    ed.handle(InputEvent::Action(Action::AlignBottom));
    let a = ed.document.get(ids[0]).unwrap().bounds().centre();
    let b = ed.document.get(ids[1]).unwrap().bounds().centre();
    assert!((b.x - a.x - before[1].x + before[0].x).abs() < 0.001);
    assert!((b.y - a.y - before[1].y + before[0].y).abs() < 0.001);
}

#[test]
fn alt_duplicate_toggles_immediately_when_modifiers_change_mid_drag() {
    let mut ed = one();
    down(&mut ed, p(150.0, 100.0), NONE);
    movement(&mut ed, p(180.0, 130.0), NONE);
    assert!(
        ed.preview()
            .iter()
            .all(|e| ed.document.get(e.id()).is_some())
    );
    ed.handle(InputEvent::Modifiers(ALT));
    assert!(
        ed.preview()
            .iter()
            .all(|e| ed.document.get(e.id()).is_none())
    );
    ed.handle(InputEvent::Modifiers(NONE));
    assert!(
        ed.preview()
            .iter()
            .all(|e| ed.document.get(e.id()).is_some())
    );
    ed.handle(InputEvent::Modifiers(ALT));
    up(&mut ed, p(180.0, 130.0), ALT);
    assert_eq!(ed.document.elements().len(), 2);
    assert_eq!(
        ed.document.elements()[0].kind.bounds(),
        crate::selection_tests::r(100.0, 100.0, 200.0, 100.0)
    );
}

#[test]
fn shift_rotation_snaps_the_absolute_angle_of_an_already_rotated_element() {
    let mut ed = one();
    let before = ed.document.elements()[0].clone();
    let mut after = before.clone();
    after.angle = 0.1;
    ed.execute(
        Command::Patch {
            id: before.id(),
            before,
            after,
        },
        ed.selection.clone(),
    );
    let bounds = ed.selection_bounds().unwrap();
    let handle = Handle::Rotate.position(bounds, 1.0);
    down(&mut ed, p(handle.x, handle.y), NONE);
    movement(&mut ed, p(handle.x + 25.0, handle.y + 4.0), NONE);
    ed.handle(InputEvent::Modifiers(SHIFT));
    let angle = ed.preview()[0].angle;
    let steps = angle / (std::f32::consts::PI / 12.0);
    assert!((steps - steps.round()).abs() < 0.001);
}

#[test]
fn flip_reflects_asymmetric_path_points_in_world_space() {
    let mut ed = Editor::default();
    let id = ed.document.alloc_id();
    let mut e = Element::new(
        id,
        Kind::Arrow(Path::from_world(&[
            WorldPt::new(100.0, 100.0),
            WorldPt::new(130.0, 170.0),
            WorldPt::new(200.0, 120.0),
        ])),
        Props::default(),
    );
    e.angle = 0.3;
    ed.execute(Command::Add { element: e, at: 0 }, Selection::default());
    ed.selection = Selection::only(id);
    let display = |e: &Element| {
        e.kind
            .path()
            .unwrap()
            .world_points()
            .map(|p| pen_core::rotate(p, e.kind.bounds().centre(), e.angle))
            .collect::<Vec<_>>()
    };
    let before = display(ed.document.get(id).unwrap());
    let centre = ed.selection_bounds().unwrap().centre();
    key(&mut ed, Key::Char('h'), SHIFT);
    let after = display(ed.document.get(id).unwrap());
    for (a, b) in before.iter().zip(after) {
        assert!((b.x - (2.0 * centre.x - a.x)).abs() < 0.001);
        assert!((b.y - a.y).abs() < 0.001);
    }
}

#[test]
fn eraser_index_finds_rotated_geometry_and_unchanged_strokes_do_not_repaint() {
    let mut ed = one();
    let before = ed.document.elements()[0].clone();
    let after = before.rotated(before.kind.bounds().centre(), std::f32::consts::FRAC_PI_2);
    ed.execute(
        Command::Patch {
            id: before.id(),
            before,
            after,
        },
        ed.selection.clone(),
    );
    key(&mut ed, Key::Char('e'), NONE);
    down(&mut ed, p(200.0, 50.0), NONE);
    assert_eq!(ed.preview().len(), 1);
    assert_eq!(
        ed.handle(InputEvent::PointerMove {
            at: p(200.0, 55.0),
            mods: NONE
        }),
        Redraw::Nothing
    );
    up(&mut ed, p(200.0, 55.0), NONE);
    assert!(ed.document.elements().is_empty());
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(ed.document.elements().len(), 1);
}
