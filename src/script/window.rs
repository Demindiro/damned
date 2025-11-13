use super::{BigInt, Compiler, Dictionary, Object, Stack, dict};
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
    let (_int, obj) = (int.clone(), obj.clone());
    dictionary.define(
        "Window",
        dict(
            read_word.clone(),
            &[(
                "print",
                comp.with(move || {
                    let x = obj.pop()?;
                    let s = String::from_utf8_lossy(x.data());
                    println!("{s}");
                    Ok(())
                }),
            )],
        ),
    );
}
