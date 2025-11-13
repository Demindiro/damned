use super::{BigInt, Compiler, Dictionary, Object, Stack, dict};
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
    let read_word = read_word.clone();
    let int = int.clone();
    let obj = obj.clone();
    let int2 = int.clone();
    let obj2 = obj.clone();
    dictionary.with(|d| {
        d.define(
            "String",
            dict(
                read_word,
                &[
                    (
                        "decimal",
                        comp.with(move || {
                            let x = int2.pop()?;
                            obj2.push(x.to_string().into())?;
                            Ok(())
                        }),
                    ),
                    (
                        "split",
                        comp.with(move || {
                            let x = int.pop()?;
                            let x = u32::try_from(x).unwrap();
                            let x = char::from_u32(x).unwrap();
                            let y = obj.pop()?;
                            let y = <&str>::try_from(&y)?;
                            let mut n = 0;
                            obj.push(y.split(&[x]).map(Object::from).collect())
                        }),
                    ),
                ],
            ),
        )
    });
}
