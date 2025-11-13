use super::{BigInt, Compiler, Dictionary, Object, Stack, Word, dict, with, with_imm};
use core::cell::Cell;
use std::rc::Rc;

pub fn define<F>(
    comp: &Compiler,
    read_word: &Rc<F>,
    d: &Dictionary,
    int: &Rc<Stack<BigInt>>,
    obj: &Rc<Stack<Object>>,
) where
    F: 'static + Clone + Fn() -> super::Result<Option<String>>,
{
    fn f<T, F>(comp: &Compiler, d: &Dictionary, read_word: &Rc<F>, stack: &Rc<Stack<T>>) -> Word
    where
        F: 'static + Fn() -> super::Result<Option<String>>,
        T: 'static + Default + Clone,
    {
        let comp = comp.clone();
        let read_word = read_word.clone();
        let s = stack.clone();
        let d = d.clone();
        with_imm(move || {
            let name = read_word()?.unwrap();
            let x = Rc::new(Cell::new(T::default()));
            let x2 = x.clone();
            let s = s.clone();
            let s2 = s.clone();
            d.with(|d| {
                d.define(
                    &name,
                    with(&comp, move || {
                        let x = x2.take();
                        x2.set(x.clone());
                        s2.push(x)
                    }),
                )
            });
            d.with(|d| {
                d.define(
                    &format!("set:{name}"),
                    with(&comp, move || s.pop().map(|v| x.set(v))),
                )
            });
            Ok(())
        })
    }
    let int = ("integer", f(comp, d, read_word, int));
    let obj = ("object", f(comp, d, read_word, obj));
    d.with(|d| d.define("Var", dict(read_word.clone(), &[int, obj])));
}
