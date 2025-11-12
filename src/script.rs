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

type Error = Box<dyn std::error::Error>;
type Result<T> = core::result::Result<T, Error>;

trait Word {
    fn eval(&self, vm: &mut Vm) -> Result<()>;

    fn is_immediate(&self) -> bool {
        false
    }

    fn is_pure(&self) -> bool {
        false
    }
}

#[derive(Default)]
struct Vm {
    streams: Vec<Box<dyn FnMut() -> Option<u8>>>,
    dictionary: Dictionary,
    compiler: Option<Compiler>,
}

#[derive(Default)]
struct Dictionary {
    words: BTreeMap<Box<str>, Rc<dyn Word>>,
    alt: Option<Box<dyn Fn(&str) -> Option<Rc<dyn Word>>>>,
}

#[derive(Default)]
struct NameSpace {
    words: BTreeMap<Box<str>, Rc<dyn Word>>,
}

#[derive(Default)]
struct Compiler {
    name: Box<str>,
    words: Vec<Rc<dyn Word>>,
}

struct WordFn<const IMMEDIATE: bool, F>(F);

#[derive(Clone, Debug, Default)]
struct Object {
    data: Box<[u8]>,
    refs: Box<[Object]>,
}

struct Stack<T> {
    stack: Cell<Vec<T>>,
}

impl Vm {
    pub fn run(&mut self) -> Result<()> {
        while let Some(x) = self.read_word()? {
            self.dictionary
                .get(&x)
                .ok_or_else(|| todo!("{x}"))
                .and_then(|x| self.eval(&x))?;
        }
        Ok(())
    }

    fn eval(&mut self, word: &Rc<dyn Word>) -> Result<()> {
        if let Some(c) = self.compiler.as_mut().filter(|_| !word.is_immediate()) {
            c.push(word.clone());
        } else {
            word.eval(self)?;
        }
        Ok(())
    }

    fn read_word(&mut self) -> Result<Option<String>> {
        let mut word = vec![];
        while let Some(x) = self.streams.last_mut().and_then(|x| x()) {
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
    }

    fn define(&mut self, word: &str, value: Rc<dyn Word>) {
        self.dictionary.define(word, value);
    }

    fn compile_begin(&mut self, name: &str) -> Result<()> {
        assert!(self.compiler.is_none());
        self.compiler = Some(Compiler::new(name));
        Ok(())
    }

    fn compile_end(&mut self) -> Result<()> {
        self.compiler.take().unwrap().finish(self);
        Ok(())
    }
}

impl Dictionary {
    fn define(&mut self, word: &str, value: Rc<dyn Word>) {
        self.words.insert(word.into(), value);
    }

    fn get(&self, word: &str) -> Option<Rc<dyn Word>> {
        if let Some(x) = self.words.get(word).cloned() {
            return Some(x);
        }
        self.alt.as_ref().and_then(|x| (x)(word))
    }

    fn push_alt<F>(&mut self, f: F)
    where
        F: 'static + Fn(&str) -> Option<Rc<dyn Word>>,
    {
        self.alt = Some(if let Some(next) = self.alt.take() {
            Box::new(move |s| (f)(s).or_else(|| (next)(s)))
        } else {
            Box::new(f)
        });
    }
}

impl Compiler {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            words: Default::default(),
        }
    }

    pub fn push(&mut self, word: Rc<dyn Word>) {
        self.words.push(word);
    }

    pub fn finish(self, vm: &mut Vm) {
        let x: Box<[_]> = self.words.into();
        let x = with(move |vm| x.iter().try_for_each(|x| vm.eval(x)));
        vm.define(&self.name, x);
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

impl<const IMMEDIATE: bool, F> Word for WordFn<IMMEDIATE, F>
where
    F: Fn(&mut Vm) -> Result<()>,
{
    fn eval(&self, vm: &mut Vm) -> Result<()> {
        (self.0)(vm)
    }

    fn is_immediate(&self) -> bool {
        IMMEDIATE
    }
}

impl Word for NameSpace {
    fn eval(&self, vm: &mut Vm) -> Result<()> {
        let word = vm.read_word()?.unwrap();
        let x = self.words.get(&*word).unwrap();
        vm.eval(x)
    }

    fn is_immediate(&self) -> bool {
        true
    }
}

/// Create VM with all capabilities.
pub fn create_root_vm<A>(args: A) -> impl FnMut(&[u8]) -> Result<()>
where
    A: IntoIterator<Item = String>,
{
    let mut vm = Vm::default();
    let def_int = define_int(&mut vm.dictionary);
    let def_obj = define_obj(&mut vm.dictionary, &def_int);
    args.into_iter()
        .for_each(|x| def_obj.push(x.into()).unwrap());
    define_compiler(&mut vm.dictionary);
    let obj = def_obj.clone();
    vm.define(
        "Window",
        dict(&[(
            "print",
            with(move |vm| {
                let x = obj.pop()?;
                let s = String::from_utf8_lossy(&x.data);
                println!("{s}");
                Ok(())
            }),
        )]),
    );
    let int = def_int.clone();
    let int2 = def_int.clone();
    let obj = def_obj.clone();
    vm.define(
        "Sys",
        dict(&[
            (
                "Fs",
                dict(&[(
                    "read",
                    with(move |vm| {
                        let x = obj.pop()?;
                        let x = <&str>::try_from(&x)?;
                        obj.push(std::fs::read(x)?.into())
                    }),
                )]),
            ),
            (
                "Terminal",
                dict(&[
                    ("wait", with(move |_| encode_event(&int2, event::read()?))),
                    (
                        "set-cursor",
                        with(move |vm| {
                            let y = int.pop()?;
                            let x = int.pop()?;
                            let y = u16::try_from(y)?;
                            let x = u16::try_from(x)?;
                            execute!(std::io::stdout(), cursor::MoveTo(x, y))?;
                            Ok(())
                        }),
                    ),
                ]),
            ),
        ]),
    );
    let int = def_int.clone();
    let int2 = def_int.clone();
    let obj = def_obj.clone();
    let obj2 = def_obj.clone();
    vm.define(
        "String",
        dict(&[
            (
                "decimal",
                with(move |vm| {
                    let x = int2.pop()?;
                    obj2.push(x.to_string().into())?;
                    Ok(())
                }),
            ),
            (
                "split",
                with(move |vm| {
                    let x = int.pop()?;
                    let x = u32::try_from(x).unwrap();
                    let x = char::from_u32(x).unwrap();
                    let y = obj.pop()?;
                    let y = <&str>::try_from(&y)?;
                    let mut n = 0;
                    obj.push(y.split(&[x]).map(Object::from).collect())
                }),
            ),
        ]),
    );
    define_global(&mut vm.dictionary, &def_int, &def_obj);
    move |s| {
        if s.is_empty() {
            return vm.run();
        }
        let mut s = Vec::from(s).into_iter();
        vm.streams.push(Box::new(move || s.next()));
        Ok(())
    }
}

/// Create a word from a closure.
fn with<F>(f: F) -> Rc<dyn Word>
where
    F: 'static + Fn(&mut Vm) -> Result<()>,
{
    Rc::new(WordFn::<false, F>(f))
}

/// Create an immediate word from a closure.
fn with_imm<F>(f: F) -> Rc<dyn Word>
where
    F: 'static + Fn(&mut Vm) -> Result<()>,
{
    Rc::new(WordFn::<true, F>(f))
}

fn dict(words: &[(&str, Rc<dyn Word>)]) -> Rc<dyn Word> {
    Rc::new(NameSpace {
        words: words
            .iter()
            .map(|(k, v)| (Box::from(*k), v.clone()))
            .collect(),
    })
}

fn define_compiler(dict: &mut Dictionary) {
    dict.define(
        ":",
        with_imm(|vm| {
            let name = vm.read_word()?.unwrap();
            vm.compile_begin(&name)
        }),
    );
    dict.define(";", with_imm(|vm| vm.compile_end()));
}

fn define_global(d: &mut Dictionary, int: &Rc<Stack<BigInt>>, obj: &Rc<Stack<Object>>) {
    fn f<T>(stack: &Rc<Stack<T>>) -> Rc<dyn Word>
    where
        T: 'static + Default + Clone,
    {
        let s = stack.clone();
        with_imm(move |vm| {
            let name = vm.read_word()?.unwrap();
            let x = Rc::new(Cell::new(T::default()));
            let x2 = x.clone();
            let s = s.clone();
            let s2 = s.clone();
            vm.define(
                &name,
                with(move |_| {
                    let x = x2.take();
                    x2.set(x.clone());
                    s2.push(x)
                }),
            );
            vm.define(
                &format!("set:{name}"),
                with(move |_| s.pop().map(|v| x.set(v))),
            );
            Ok(())
        })
    }
    d.define("Global", dict(&[("integer", f(int)), ("object", f(obj))]));
}

fn define_int(dict: &mut Dictionary) -> Rc<Stack<BigInt>> {
    fn f<T, F>(stack: &Rc<Stack<T>>, dict: &mut Dictionary, name: &str, f: F)
    where
        F: 'static + Fn(&Stack<T>) -> Result<()> + 'static,
        // TODO why 'static?
        T: 'static,
    {
        let stack = stack.clone();
        dict.define(name, with(move |_| (f)(&stack)));
    }
    let mut stack = Rc::new(Stack::<BigInt>::default());
    let s = &mut stack;
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
    dict.push_alt(move |name| {
        let f = |x: BigInt| {
            let s = s.clone();
            with(move |_| s.push(x.clone()))
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

fn define_obj(dict: &mut Dictionary, int: &Rc<Stack<BigInt>>) -> Rc<Stack<Object>> {
    fn f<T, F>(stack: &Rc<Stack<T>>, dict: &mut Dictionary, name: &str, f: F)
    where
        F: 'static + Fn(&Stack<T>) -> Result<()> + 'static,
        // TODO why 'static?
        T: 'static,
    {
        let stack = stack.clone();
        dict.define(name, with(move |_| (f)(&stack)));
    }
    let mut stack = Rc::new(Stack::<Object>::default());
    let s = &mut stack;
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
