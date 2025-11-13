use super::{BigInt, Compiler, Dictionary, Object, Stack};
use crossterm::terminal;
use std::rc::Rc;

pub fn define<F>(
    comp: &Compiler,
    dictionary: &Dictionary,
    read_word: &Rc<F>,
    int: &Rc<Stack<BigInt>>,
    obj: &Rc<Stack<Object>>,
) where
    F: 'static + Clone + Fn() -> super::Result<Option<String>>,
{
    let (int, obj) = (int.clone(), obj.clone());
    dictionary.dict(
        "Window",
        read_word,
        &[
            (
                "print",
                comp.with(move || {
                    let x = obj.pop()?;
                    let s = String::from_utf8_lossy(x.data());
                    println!("{s}");
                    Ok(())
                }),
            ),
            (
                "size",
                comp.with(move || {
                    let (x, y) = terminal::size()?;
                    int.push(x.into())?;
                    int.push(y.into())
                }),
            ),
        ],
    );
}
