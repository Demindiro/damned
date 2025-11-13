use super::{BigInt, Compiler, Dictionary, Object, Stack, dict};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
};
use std::rc::Rc;

pub fn define<F>(
    comp: &Compiler,
    dictionary: &Dictionary,
    read_word: &Rc<F>,
    int: &Rc<Stack<BigInt>>,
    obj: &Rc<Stack<Object>>,
) where
    F: 'static + Fn() -> super::Result<Option<String>>,
{
    let int = int.clone();
    let obj = obj.clone();
    let int2 = int.clone();
    dictionary.dict(
        "Sys",
        read_word,
        &[
            (
                "Fs",
                dict(
                    read_word.clone(),
                    &[(
                        "read",
                        comp.with(move || {
                            let x = obj.pop()?;
                            let x = <&str>::try_from(&x)?;
                            obj.push(std::fs::read(x)?.into())
                        }),
                    )],
                ),
            ),
            (
                "Terminal",
                dict(
                    read_word.clone(),
                    &[
                        (
                            "wait",
                            comp.with(move || encode_event(&int2, event::read()?)),
                        ),
                        (
                            "set-cursor",
                            comp.with(move || {
                                let y = int.pop()?;
                                let x = int.pop()?;
                                let y = u16::try_from(y)?;
                                let x = u16::try_from(x)?;
                                execute!(std::io::stdout(), cursor::MoveTo(x, y))?;
                                Ok(())
                            }),
                        ),
                    ],
                ),
            ),
        ],
    );
}

fn encode_event(int: &Stack<BigInt>, event: Event) -> super::Result<()> {
    match event {
        Event::FocusGained => todo!(),
        Event::FocusLost => todo!(),
        Event::Key(x) => int.push(encode_key_event(x).into()),
        Event::Mouse(x) => todo!("{x:?}"),
        Event::Paste(s) => todo!("{s:?}"),
        Event::Resize(x, y) => todo!("{x} {y}"),
    }
}

fn encode_key_event(key: KeyEvent) -> i32 {
    let KeyEvent {
        code,
        modifiers,
        kind,
        state: _,
    } = key;
    let mut x = match code {
        KeyCode::Backspace => todo!(),
        KeyCode::Enter => todo!(),
        KeyCode::Left => todo!(),
        KeyCode::Right => todo!(),
        KeyCode::Up => todo!(),
        KeyCode::Down => todo!(),
        KeyCode::PageUp => todo!(),
        KeyCode::PageDown => todo!(),
        KeyCode::Home => todo!(),
        KeyCode::End => todo!(),
        KeyCode::Tab => todo!(),
        KeyCode::BackTab => todo!(),
        KeyCode::Delete => todo!(),
        KeyCode::Insert => todo!(),
        KeyCode::Null => todo!(),
        KeyCode::Esc => todo!(),
        KeyCode::CapsLock => todo!(),
        KeyCode::ScrollLock => todo!(),
        KeyCode::NumLock => todo!(),
        KeyCode::PrintScreen => todo!(),
        KeyCode::Pause => todo!(),
        KeyCode::Menu => todo!(),
        KeyCode::KeypadBegin => todo!(),
        KeyCode::Media(x) => todo!("{x:?}"),
        KeyCode::Modifier(x) => todo!("{x:?}"),
        KeyCode::F(x) => todo!("{x:?}"),
        KeyCode::Char(x) => x as i32,
    };
    let mut f = |m, s| x |= i32::from(modifiers.contains(m)) << (21 + s);
    f(KeyModifiers::SHIFT, 0);
    f(KeyModifiers::CONTROL, 1);
    f(KeyModifiers::ALT, 2);
    f(KeyModifiers::SUPER, 3);
    f(KeyModifiers::HYPER, 4);
    f(KeyModifiers::META, 5);
    x |= (match kind {
        KeyEventKind::Press => 0b01,
        KeyEventKind::Release => 0b10,
        KeyEventKind::Repeat => 0b11,
    }) << (21 + 6);
    x
}
