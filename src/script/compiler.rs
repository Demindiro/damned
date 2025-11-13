use super::{BigInt, Dictionary, Stack, Word, with_imm};
use std::rc::Rc;
use with_cell::WithCell;

#[derive(Clone)]
pub struct Compiler(Rc<WithCell<Option<CompilerData>>>);

struct CompilerData {
    name: Box<str>,
    words: Vec<Word>,
    cond: Option<Cond>,
}

#[derive(Default)]
struct Cond {
    cond: Vec<Word>,
    tru: Vec<Word>,
    fals: Vec<Word>,
    stage: CondStage,
}

#[derive(Default)]
enum CondStage {
    #[default]
    Cond,
    True,
    False,
}

impl CompilerData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            words: Default::default(),
            cond: None,
        }
    }

    pub fn push(&mut self, word: Word) {
        let v = if let Some(c) = self.cond.as_mut() {
            match &c.stage {
                CondStage::Cond => &mut c.cond,
                CondStage::True => &mut c.tru,
                CondStage::False => &mut c.fals,
            }
        } else {
            &mut self.words
        };
        v.push(word);
    }
}

impl Cond {
    fn finish(self) -> super::Result<[Box<[Word]>; 3]> {
        let Self {
            cond,
            tru,
            fals,
            stage,
        } = self;
        assert!(matches!(stage, CondStage::True | CondStage::False));
        Ok([cond, tru, fals].map(|x| x.into_boxed_slice()))
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

    fn finish(&self, dict: &Dictionary) -> Option<Word> {
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

    fn cond_begin(&self) -> super::Result<()> {
        self.0.with(|c| {
            let c = c.get_or_insert_with(|| CompilerData::new(""));
            assert!(c.cond.is_none(), "can't nest conditions");
            c.cond = Some(Cond::default());
            Ok(())
        })
    }

    fn cond_then(&self) -> super::Result<()> {
        self.0.with(|c| {
            let c = c.as_mut().unwrap();
            let c = c.cond.as_mut().unwrap();
            assert!(matches!(&c.stage, CondStage::Cond));
            c.stage = CondStage::True;
            Ok(())
        })
    }

    fn cond_else(&self) -> super::Result<()> {
        self.0.with(|c| {
            let c = c.as_mut().unwrap();
            let c = c.cond.as_mut().unwrap();
            assert!(matches!(&c.stage, CondStage::True));
            c.stage = CondStage::False;
            Ok(())
        })
    }

    fn cond_end(&self, stack: &Rc<Stack<BigInt>>) -> super::Result<()> {
        let f = self.0.with(|cc| {
            let c = cc.as_mut().unwrap();
            let [cond, tru, fals] = c.cond.take().unwrap().finish()?;
            let stack = stack.clone();
            let f = move || {
                cond.iter().try_for_each(|x| (x)())?;
                (stack.pop()? != BigInt::ZERO)
                    .then_some(&tru)
                    .unwrap_or(&fals)
                    .iter()
                    .try_for_each(|x| (x)())
            };
            super::Result::Ok(if c.name.is_empty() {
                *cc = None;
                Some(f)
            } else {
                c.push(with_imm(f));
                None
            })
        })?;
        f.map(|f| (f)()).transpose().map(|_| ())
    }

    fn cond_repeat(&self, stack: &Rc<Stack<BigInt>>) -> super::Result<()> {
        let f = self.0.with(|cc| {
            let c = cc.as_mut().unwrap();
            let [cond, tru, fals] = c.cond.take().unwrap().finish()?;
            let stack = stack.clone();
            let f = move || {
                while {
                    cond.iter().try_for_each(|x| (x)())?;
                    stack.pop()? != BigInt::ZERO
                } {
                    tru.iter().try_for_each(|x| (x)())?;
                }
                fals.iter().try_for_each(|x| (x)())
            };
            super::Result::Ok(if c.name.is_empty() {
                *cc = None;
                Some(f)
            } else {
                c.push(with_imm(f));
                None
            })
        })?;
        f.map(|f| (f)()).transpose().map(|_| ())
    }
}

pub fn define<F>(read_word: Rc<F>, dict: &Dictionary, stack: &Rc<Stack<BigInt>>) -> Compiler
where
    F: 'static + Fn() -> super::Result<Option<String>>,
{
    let compiler = Compiler(Default::default());
    let c = compiler.clone();
    dict.imm(":", move || {
        assert!(c.0.take().is_none(), "todo: already compiling");
        let name = read_word()?.unwrap();
        assert!(!name.is_empty(), "todo: forbid empty names");
        c.0.set(Some(CompilerData::new(&name)));
        Ok(())
    });
    let c = compiler.clone();
    let d = dict.clone();
    dict.imm(";", move || {
        c.finish(&d).map(|x| (x)()).transpose().map(|_| ())
    });
    let c = compiler.clone();
    dict.imm("if", move || c.cond_begin());
    let c = compiler.clone();
    dict.imm("then", move || c.cond_then());
    let c = compiler.clone();
    dict.imm("else", move || c.cond_else());
    let c = compiler.clone();
    let s = stack.clone();
    dict.imm("end", move || c.cond_end(&s));
    let c = compiler.clone();
    let s = stack.clone();
    dict.imm("repeat", move || c.cond_repeat(&s));
    compiler
}
