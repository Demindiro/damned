mod string;
mod var;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
};
use num::BigInt;
use std::{
    cell::Cell,
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    rc::Rc,
};
use with_cell::WithCell;

type Error = Box<dyn std::error::Error>;
type Result<T> = core::result::Result<T, Error>;

type Word = Rc<dyn Fn() -> Result<()>>;

#[derive(Default)]
struct DictionaryData {
    words: BTreeMap<Box<str>, Word>,
    alt: Option<Box<dyn Fn(&str) -> Option<Word>>>,
}

type Dictionary = Rc<WithCell<DictionaryData>>;

#[derive(Default)]
struct NameSpace {
    words: BTreeMap<Box<str>, Word>,
}

struct CompilerData {
    name: Box<str>,
    words: Vec<Word>,
}

#[derive(Clone, Default)]
struct Compiler(Rc<WithCell<Option<CompilerData>>>);

#[derive(Clone, Debug, Default)]
struct Object {
    data: Box<[u8]>,
    refs: Box<[Object]>,
}

struct Stack<T> {
    stack: Cell<Vec<T>>,
}

impl DictionaryData {
    fn define(&mut self, word: &str, value: Word) {
        self.words.insert(word.into(), value);
    }

    fn get(&self, word: &str) -> Option<Word> {
        if let Some(x) = self.words.get(word).cloned() {
            return Some(x);
        }
        self.alt.as_ref().and_then(|x| (x)(word))
    }

    fn push_alt<F>(&mut self, f: F)
    where
        F: 'static + Fn(&str) -> Option<Word>,
    {
        self.alt = Some(if let Some(next) = self.alt.take() {
            Box::new(move |s| (f)(s).or_else(|| (next)(s)))
        } else {
            Box::new(f)
        });
    }
}

impl CompilerData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            words: Default::default(),
        }
    }

    pub fn push(&mut self, word: Word) {
        self.words.push(word);
    }
}

impl Compiler {
    /// Create a word from a closure.
    fn with<F>(&self, f: F) -> Word
    where
        F: 'static + Fn() -> Result<()>,
    {
        fn dyn_with(compiler: Compiler, f: Word) -> Word {
            with_imm(move || {
                // so much for WithCell...
                if let Some(mut c) = compiler.take() {
                    c.push(f.clone());
                    compiler.set(Some(c));
                    Ok(())
                } else {
                    (f)()
                }
            })
        }
        dyn_with(self.clone(), with_imm(f))
    }

    fn finish(&self, dict: &mut DictionaryData) {
        let c = self.0.with(|x| x.take()).unwrap();
        let x: Box<[_]> = c.words.into();
        let x = self.with(move || x.iter().try_for_each(|x| (x)()));
        dict.define(&c.name, x);
    }
}

impl<T> Stack<T> {
    fn with<F, E>(&self, f: F) -> E
    where
        F: FnOnce(&mut Vec<T>) -> E,
    {
        let mut v = self.stack.take();
        let res = (f)(&mut v);
        self.stack.set(v);
        res
    }

    fn push(&self, value: T) -> Result<()> {
        self.with(|v| v.push(value));
        Ok(())
    }

    fn pop(&self) -> Result<T> {
        self.with(|v| v.pop().ok_or_else(|| todo!()))
    }

    fn op2to1<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(T, T) -> T,
    {
        let y = self.pop()?;
        let x = self.pop()?;
        self.push((f)(x, y))
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self {
            stack: Default::default(),
        }
    }
}

impl From<Box<[u8]>> for Object {
    fn from(data: Box<[u8]>) -> Self {
        Self {
            data,
            refs: [].into(),
        }
    }
}

impl From<Vec<u8>> for Object {
    fn from(s: Vec<u8>) -> Self {
        s.into_boxed_slice().into()
    }
}

impl From<String> for Object {
    fn from(s: String) -> Self {
        s.into_bytes().into()
    }
}

impl From<&str> for Object {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl FromIterator<Self> for Object {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        Self {
            data: [].into(),
            refs: iter.into_iter().collect(),
        }
    }
}

impl<'a> TryFrom<&'a Object> for &'a str {
    type Error = core::str::Utf8Error;

    fn try_from(obj: &'a Object) -> core::result::Result<Self, Self::Error> {
        core::str::from_utf8(&obj.data)
    }
}

impl core::ops::Deref for Compiler {
    type Target = WithCell<Option<CompilerData>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Create VM with all capabilities.
pub fn create_root_vm<A>(args: A) -> impl FnMut(&[u8]) -> Result<()>
where
    A: IntoIterator<Item = String>,
{
    let streams = Rc::<WithCell<Vec<Box<dyn FnMut() -> Option<u8>>>>>::default();
    let dictionary = Dictionary::default();

    let s = streams.clone();
    let read_word = Rc::new(move || -> Result<Option<String>> {
        let mut word = vec![];
        while let Some(x) = s.with(|s| s.last_mut().and_then(|x| (x)())) {
            if x.is_ascii_whitespace() {
                if word.is_empty() {
                    continue;
                }
                break;
            }
            word.push(x);
        }
        (!word.is_empty())
            .then(|| Ok(String::from_utf8(word)?))
            .transpose()
    });

    let comp = &define_compiler(read_word.clone(), &dictionary);
    let def_int = dictionary.with(|d| define_int(comp, d));
    let def_obj = dictionary.with(|d| define_obj(comp, d, &def_int));
    args.into_iter()
        .for_each(|x| def_obj.push(x.into()).unwrap());
    let obj = def_obj.clone();
    dictionary.with(|d| {
        d.define(
            "Window",
            dict(
                read_word.clone(),
                &[(
                    "print",
                    comp.with(move || {
                        let x = obj.pop()?;
                        let s = String::from_utf8_lossy(&x.data);
                        println!("{s}");
                        Ok(())
                    }),
                )],
            ),
        )
    });
    let int = def_int.clone();
    let int2 = def_int.clone();
    let obj = def_obj.clone();
    dictionary.with(|d| {
        d.define(
            "Sys",
            dict(
                read_word.clone(),
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
            ),
        )
    });
    string::define(comp, &dictionary, &read_word, &def_int, &def_obj);
    var::define(comp, &read_word, &dictionary, &def_int, &def_obj);

    move |s| {
        if !s.is_empty() {
            let mut s = Vec::from(s).into_iter();
            streams.with(|x| x.push(Box::new(move || s.next())));
            return Ok(());
        }
        while let Some(x) = read_word()? {
            let x = dictionary.with(|d| d.get(&x)).ok_or_else(|| todo!("{x}"))?;
            (x)()?;
        }
        Ok(())
    }
}

/// Create an immediate word from a closure.
fn with_imm<F>(f: F) -> Word
where
    F: 'static + Fn() -> Result<()>,
{
    Rc::new(f)
}

fn dict<F>(read_word: Rc<F>, words: &[(&str, Word)]) -> Word
where
    F: 'static + Fn() -> Result<Option<String>>,
{
    let words = words
        .iter()
        .map(|(k, v)| (Box::from(*k), v.clone()))
        .collect::<BTreeMap<_, _>>();
    with_imm(move || {
        let word = read_word()?.unwrap();
        let x = words.get(&*word).unwrap();
        (x)()
    })
}

fn define_compiler<F>(read_word: Rc<F>, dict: &Dictionary) -> Compiler
where
    F: 'static + Fn() -> Result<Option<String>>,
{
    let mut compiler = Compiler::default();
    let c = compiler.clone();
    dict.with(|d| {
        d.define(
            ":",
            with_imm(move || {
                let name = read_word()?.unwrap();
                assert!(c.take().is_none(), "todo: already compiling");
                c.set(Some(CompilerData::new(&name)));
                Ok(())
            }),
        )
    });
    let c = compiler.clone();
    let d = dict.clone();
    dict.with(|dict| dict.define(";", with_imm(move || d.with(|d| Ok(c.finish(d))))));
    compiler
}

fn define_int(comp: &Compiler, dict: &mut DictionaryData) -> Rc<Stack<BigInt>> {
    fn f<T, F>(
        (comp, stack): (&Compiler, &Rc<Stack<T>>),
        dict: &mut DictionaryData,
        name: &str,
        f: F,
    ) where
        F: 'static + Fn(&Stack<T>) -> Result<()> + 'static,
        // TODO why 'static?
        T: 'static,
    {
        let stack = stack.clone();
        dict.define(name, comp.with(move || (f)(&stack)));
    }
    let stack = Rc::new(Stack::<BigInt>::default());
    let s = (comp, &stack);
    f(s, dict, "+", |s| s.op2to1(|x, y| x + y));
    f(s, dict, "-", |s| s.op2to1(|x, y| x - y));
    f(s, dict, "*", |s| s.op2to1(|x, y| x * y));
    f(s, dict, "=", |s| s.op2to1(|x, y| (x == y).into()));
    f(s, dict, "<>", |s| s.op2to1(|x, y| (x != y).into()));
    f(s, dict, "<", |s| s.op2to1(|x, y| (x < y).into()));
    f(s, dict, ">", |s| s.op2to1(|x, y| (x > y).into()));
    f(s, dict, "<=", |s| s.op2to1(|x, y| (x <= y).into()));
    f(s, dict, ">=", |s| s.op2to1(|x, y| (x >= y).into()));
    f(s, dict, "#dup", |s| {
        let x = s.pop()?;
        s.push(x.clone())?;
        s.push(x)
    });
    f(s, dict, "#drop", |s| s.pop().map(|_| ()));
    f(s, dict, "#swap", |s| {
        let x = s.pop()?;
        let y = s.pop()?;
        s.push(x)?;
        s.push(y)
    });
    let s = stack.clone();
    let comp = comp.clone();
    dict.push_alt(move |name| {
        let f = |x: BigInt| {
            let s = s.clone();
            comp.with(move || s.push(x.clone()))
        };
        if name.len() > 2 && name.starts_with("'") && name.ends_with("'") {
            let mut it = name.chars().skip(1);
            let c = match it.next().unwrap() {
                '\\' => match it.next().unwrap() {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    c => todo!("{c:?}"),
                },
                c => c,
            };
            assert_eq!(it.next().unwrap(), '\'');
            return Some(f(BigInt::from(c as u32)));
        }
        name.parse::<BigInt>().ok().map(f)
    });
    stack
}

fn define_obj(
    comp: &Compiler,
    dict: &mut DictionaryData,
    int: &Rc<Stack<BigInt>>,
) -> Rc<Stack<Object>> {
    fn f<T, F>(
        (comp, stack): (&Compiler, &Rc<Stack<T>>),
        dict: &mut DictionaryData,
        name: &str,
        f: F,
    ) where
        F: 'static + Fn(&Stack<T>) -> Result<()> + 'static,
        // TODO why 'static?
        T: 'static,
    {
        let stack = stack.clone();
        dict.define(name, comp.with(move || (f)(&stack)));
    }
    let stack = Rc::new(Stack::<Object>::default());
    let s = (comp, &stack);
    f(s, dict, "@dup", |s| {
        let x = s.pop()?;
        s.push(x.clone())?;
        s.push(x)
    });
    f(s, dict, "@drop", |s| s.pop().map(|_| ()));
    f(s, dict, "@swap", |s| {
        let x = s.pop()?;
        let y = s.pop()?;
        s.push(x)?;
        s.push(y)
    });
    let int2 = int.clone();
    f(s, dict, "@data", move |s| {
        let i = int2.pop()?;
        let i = usize::try_from(i).unwrap();
        let x = *s.pop()?.data.get(i).unwrap();
        int2.push(x.into())
    });
    let int2 = int.clone();
    f(s, dict, "@refs", move |s| {
        let i = int2.pop()?;
        let i = usize::try_from(i).unwrap();
        let x = s.pop()?.refs.get(i).unwrap().clone();
        s.push(x.into())
    });
    stack
}

fn encode_event(int: &Stack<BigInt>, event: Event) -> Result<()> {
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
        state,
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
