use super::{BigInt, Compiler, Dictionary, Object, Stack, Word, dict};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue, terminal,
};
use std::rc::Rc;

const KEY_ARROW_UP: i32 = (0b11 << 19) | 0b00;
const KEY_ARROW_DOWN: i32 = (0b11 << 19) | 0b01;
const KEY_ARROW_LEFT: i32 = (0b11 << 19) | 0b10;
const KEY_ARROW_RIGHT: i32 = (0b11 << 19) | 0b11;
const KEY_DELETE: i32 = (0b11 << 19) | 0x10;
const KEY_BACKSPACE: i32 = (0b11 << 19) | 0x11;

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
    let obj2 = obj.clone();
    dictionary.dict(
        "Sys",
        read_word,
        &[
            ("Terminal", define_terminal(comp, read_word, &int, &obj)),
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
                "panic",
                comp.with(move || {
                    let msg = obj2.pop()?;
                    let msg = String::from_utf8_lossy(msg.data());
                    let msg = format!("{msg}");
                    Err(msg.into())
                }),
            ),
        ],
    );
}

fn define_terminal<F>(
    comp: &Compiler,
    read_word: &Rc<F>,
    int: &Rc<Stack<BigInt>>,
    obj: &Rc<Stack<Object>>,
) -> Word
where
    F: 'static + Fn() -> super::Result<Option<String>>,
{
    let (int, obj) = (int.clone(), obj.clone());
    let int = int.clone();
    let int2 = int.clone();
    let int3 = int.clone();
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
                    queue!(std::io::stdout(), cursor::MoveTo(x, y))?;
                    Ok(())
                }),
            ),
            (
                "clear",
                comp.with(move || {
                    Ok(queue!(
                        std::io::stdout(),
                        terminal::Clear(terminal::ClearType::All)
                    )?)
                }),
            ),
            (
                "clear-line",
                comp.with(move || {
                    Ok(queue!(
                        std::io::stdout(),
                        terminal::Clear(terminal::ClearType::CurrentLine)
                    )?)
                }),
            ),
            (
                "print",
                comp.with(move || {
                    let x = obj.pop()?;
                    let s = String::from_utf8_lossy(x.data());
                    use std::io::Write;
                    std::io::stdout().write_all(s.as_bytes())?;
                    Ok(())
                }),
            ),
            (
                "size",
                comp.with(move || {
                    let (x, y) = terminal::size()?;
                    int3.push(x.into())?;
                    int3.push(y.into())
                }),
            ),
            ("flush", comp.with(move || Ok(execute!(std::io::stdout())?))),
        ],
    )
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
        KeyCode::Backspace => KEY_BACKSPACE,
        KeyCode::Enter => '\n' as i32,
        KeyCode::Left => KEY_ARROW_LEFT,
        KeyCode::Right => KEY_ARROW_RIGHT,
        KeyCode::Up => KEY_ARROW_UP,
        KeyCode::Down => KEY_ARROW_DOWN,
        KeyCode::PageUp => todo!(),
        KeyCode::PageDown => todo!(),
        KeyCode::Home => todo!(),
        KeyCode::End => todo!(),
        KeyCode::Tab => todo!(),
        KeyCode::BackTab => todo!(),
        KeyCode::Delete => KEY_DELETE,
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
