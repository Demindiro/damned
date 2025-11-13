mod compiler;
mod int;
mod object;
mod string;
mod sys;
mod var;
mod window;

use compiler::Compiler;
use num::BigInt;
use object::Object;
use std::{cell::Cell, collections::BTreeMap, rc::Rc};
use with_cell::WithCell;

type Error = Box<dyn std::error::Error>;
type Result<T> = core::result::Result<T, Error>;

type Word = Rc<dyn Fn() -> Result<()>>;

#[derive(Default)]
struct DictionaryData {
    words: BTreeMap<Box<str>, Word>,
    alt: Option<Box<dyn Fn(&str) -> Option<Word>>>,
}

#[derive(Clone)]
struct Dictionary(Rc<WithCell<DictionaryData>>);

struct Stack<T> {
    stack: Cell<Vec<T>>,
}

impl Dictionary {
    fn define(&self, word: &str, value: Word) {
        self.0.with(|d| d.words.insert(word.into(), value));
    }

    fn get(&self, word: &str) -> Option<Word> {
        self.0.with(|d| {
            if let Some(x) = d.words.get(word).cloned() {
                return Some(x);
            }
            d.alt.as_ref().and_then(|x| (x)(word))
        })
    }

    fn push_alt<F>(&self, f: F)
    where
        F: 'static + Fn(&str) -> Option<Word>,
    {
        self.0.with(|d| {
            d.alt = Some(if let Some(next) = d.alt.take() {
                Box::new(move |s| (f)(s).or_else(|| (next)(s)))
            } else {
                Box::new(f)
            });
        })
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

/// Create VM with all capabilities.
pub fn create_root_vm<A>(args: A) -> impl FnMut(&[u8]) -> Result<()>
where
    A: IntoIterator<Item = String>,
{
    let streams = Rc::<WithCell<Vec<Box<dyn FnMut() -> Option<u8>>>>>::default();
    let dictionary = Dictionary(Default::default());

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

    let comp = &compiler::define(read_word.clone(), &dictionary);
    let def_int = int::define(comp, &dictionary);
    let def_obj = object::define(comp, &dictionary, &def_int);
    args.into_iter()
        .for_each(|x| def_obj.push(x.into()).unwrap());
    window::define(comp, &dictionary, &read_word, &def_int, &def_obj);
    sys::define(comp, &dictionary, &read_word, &def_int, &def_obj);
    string::define(comp, &dictionary, &read_word, &def_int, &def_obj);
    var::define(comp, &read_word, &dictionary, &def_int, &def_obj);

    move |s| {
        if !s.is_empty() {
            let mut s = Vec::from(s).into_iter();
            streams.with(|x| x.push(Box::new(move || s.next())));
            return Ok(());
        }
        while let Some(x) = read_word()? {
            let x = dictionary.get(&x).ok_or_else(|| todo!("{x}"))?;
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
