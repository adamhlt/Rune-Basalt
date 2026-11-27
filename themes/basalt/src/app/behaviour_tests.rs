//! Components driven through Rune's runtime: real presses and keys, then the state checked.

use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, col, row};

use super::*;

#[derive(Default)]
struct S {
    n: u32,
    on: bool,
    v: f32,
    tree: TreeState,
    picked: String,
    segmented: usize,
    steps: i32,
}

fn nodes() -> Vec<Node> {
    vec![Node::branch("root", "root", vec![Node::leaf("a", "a"), Node::leaf("b", "b")]), Node::leaf("z", "z")]
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    col().padding(20.0)
        .gap(12.0)
        .items_start()
        .child(row().key("btn").child(button("Go", Kind::Primary, None, None, |s: &mut S| s.n += 1)))
        .child(row().key("btn-off").child(disabled(true, || button("Go", Kind::Primary, None, None, |s: &mut S| s.n += 100))))
        .child(row().key("check").child(checkbox("x", s.on, |s: &mut S| s.on = !s.on)))
        .child(row().key("switch").w(200.0).child(switch("x", s.on, |s: &mut S| s.on = !s.on)))
        .child(row().key("slider").child(slider(s.v, "v", 300.0, |s: &mut S, v| s.v = v)))
        .child(row().key("tree").w(240.0).child(tree(&nodes(), &s.tree, |s: &mut S| &mut s.tree, |s: &mut S, id| s.picked = id.to_owned())))
        .child(row().key("seg").child(segmented(&["One", "Two", "Three"], s.segmented, |s: &mut S, i| s.segmented = i)))
        .child(row().key("step").w(200.0).child(stepper("5", "x", |s: &mut S, d| s.steps += d)))
}

fn rt() -> Runtime<S> {
    let mut rt = Runtime::new(S::default(), view, (600.0, 700.0), Env { system_dark: true, scale: 1.0 });
    settle(&mut rt);
    rt
}

fn settle(rt: &mut Runtime<S>) {
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }
}

fn click(rt: &mut Runtime<S>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    settle(rt);
}

fn press(rt: &mut Runtime<S>, key: Key) {
    rt.key(KeyEvent { key, mods: Mods::default() });
    settle(rt);
}

#[test]
fn a_button_fires_and_a_disabled_one_does_not() {
    let mut rt = rt();
    click(&mut rt, "btn");
    assert_eq!(rt.state.n, 1);
    click(&mut rt, "btn-off");
    assert_eq!(rt.state.n, 1, "the disabled button's handler was dropped");
}

#[test]
fn checkbox_and_switch_toggle() {
    let mut rt = rt();
    click(&mut rt, "check");
    assert!(rt.state.on);
    click(&mut rt, "switch");
    assert!(!rt.state.on);
}

#[test]
fn slider_moves_with_the_arrow_keys() {
    let mut rt = rt();
    click(&mut rt, "slider");
    let after_click = rt.state.v;
    assert!(after_click > 0.0, "a press sets the value from the pointer");
    press(&mut rt, Key::Right);
    assert!((rt.state.v - (after_click + 0.05).min(1.0)).abs() < 1e-3, "Right adds 5%");
    press(&mut rt, Key::End);
    assert_eq!(rt.state.v, 1.0);
    press(&mut rt, Key::Home);
    assert_eq!(rt.state.v, 0.0);
}

#[test]
fn tree_expands_and_moves_with_the_keyboard() {
    let mut rt = rt();
    click(&mut rt, "tree-root");
    assert_eq!(rt.state.tree.selected.as_deref(), Some("root"));
    assert_eq!(rt.state.picked, "root");
    assert!(!rt.state.tree.expanded.contains(&"root".to_owned()));
    press(&mut rt, Key::Right);
    assert!(rt.state.tree.expanded.contains(&"root".to_owned()), "Right opens the branch");
    press(&mut rt, Key::Down);
    assert_eq!(rt.state.tree.selected.as_deref(), Some("a"), "Down moves into it");
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Down);
    assert_eq!(rt.state.tree.selected.as_deref(), Some("z"));
    press(&mut rt, Key::Down);
    assert_eq!(rt.state.tree.selected.as_deref(), Some("z"), "and stops at the end");
}

#[test]
fn segmented_and_stepper_respond() {
    let mut rt = rt();
    click(&mut rt, "seg");
    assert_eq!(rt.state.segmented, 1, "the control's centre is its middle segment");
    // The stepper is narrower than its 200-point wrapper: its "−" button is left of the centre.
    let (x, y) = rt.find("step").unwrap();
    rt.pointer_down(x - 80.0, y);
    rt.pointer_up(x - 80.0, y);
    settle(&mut rt);
    assert_eq!(rt.state.steps, -1, "the − button steps down");
    press(&mut rt, Key::Up);
    assert_eq!(rt.state.steps, 0, "Up on the focused stepper steps it up");
    press(&mut rt, Key::Right);
    assert_eq!(rt.state.steps, 1);
}
