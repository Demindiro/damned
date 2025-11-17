use super::{BigInt, Compiler, Dictionary, Object, Stack};
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
    let int = int.clone();
    let obj = obj.clone();
    let int2 = int.clone();
    let obj2 = obj.clone();
    let obj3 = obj.clone();
    dictionary.dict(
        "String",
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
                    obj.push(y.split(&[x]).map(Object::from).collect())
                }),
            ),
        ],
    );
    let comp = comp.clone();
    dictionary.push_alt(move |name| {
        (name.len() > 2 && name.starts_with("\"") && name.ends_with("\"")).then(|| {
            // TODO escape string
            let x = Object::from(&name[1..name.len() - 1]);
            let obj3 = obj3.clone();
            comp.with(move || obj3.push(x.clone()))
        })
    });
}
