use super::{Dictionary, DictionaryData, Word, with_imm};
use std::rc::Rc;
use with_cell::WithCell;

#[derive(Clone)]
pub struct Compiler(Rc<WithCell<Option<CompilerData>>>);

struct CompilerData {
    name: Box<str>,
    words: Vec<Word>,
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
    pub fn with<F>(&self, f: F) -> Word
    where
        F: 'static + Fn() -> super::Result<()>,
    {
        fn dyn_with(compiler: Compiler, f: Word) -> Word {
            with_imm(move || {
                // so much for WithCell...
                if let Some(mut c) = compiler.0.take() {
                    c.push(f.clone());
                    compiler.0.set(Some(c));
                    Ok(())
                } else {
                    (f)()
                }
            })
        }
        dyn_with(self.clone(), with_imm(f))
    }

    fn finish(&self, dict: &mut DictionaryData) -> Option<Word> {
        let c = self.0.with(|x| x.take()).unwrap();
        let x: Box<[_]> = c.words.into();
        let x = self.with(move || x.iter().try_for_each(|x| (x)()));
        if c.name.is_empty() {
            Some(x)
        } else {
            dict.define(&c.name, x);
            None
        }
    }
}

pub fn define<F>(read_word: Rc<F>, dict: &Dictionary) -> Compiler
where
    F: 'static + Fn() -> super::Result<Option<String>>,
{
    let compiler = Compiler(Default::default());
    let c = compiler.clone();
    dict.with(|d| {
        d.define(
            ":",
            with_imm(move || {
                assert!(c.0.take().is_none(), "todo: already compiling");
                let name = read_word()?.unwrap();
                assert!(!name.is_empty(), "todo: forbid empty names");
                c.0.set(Some(CompilerData::new(&name)));
                Ok(())
            }),
        )
    });
    let c = compiler.clone();
    let d = dict.clone();
    dict.with(|dict| {
        dict.define(
            ";",
            with_imm(move || d.with(|d| c.finish(d).map(|x| (x)()).transpose().map(|_| ()))),
        )
    });
    compiler
}
