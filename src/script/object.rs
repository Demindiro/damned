use super::{BigInt, Compiler, DictionaryData, Stack};
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct Object {
    data: Box<[u8]>,
    refs: Box<[Object]>,
}

impl Object {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn refs(&self) -> &[Object] {
        &self.refs
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

pub fn define(
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
        F: 'static + Fn(&Stack<T>) -> super::Result<()> + 'static,
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
