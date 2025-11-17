use super::{BigInt, Compiler, Dictionary, Stack};
use core::ops::Range;
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

    pub fn concat(&self, rhs: &Self) -> Self {
        Self {
            data: self.data().iter().chain(rhs.data()).cloned().collect(),
            refs: self.refs().iter().chain(rhs.refs()).cloned().collect(),
        }
    }

    pub fn slice(&self, data: Range<usize>, refs: Range<usize>) -> Self {
        Self {
            data: self.data()[data].iter().cloned().collect(),
            refs: self.refs()[refs].iter().cloned().collect(),
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

impl<const N: usize> From<[Object; N]> for Object {
    fn from(refs: [Object; N]) -> Self {
        Self {
            data: [].into(),
            refs: refs.into(),
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

pub fn define(comp: &Compiler, dict: &Dictionary, int: &Rc<Stack<BigInt>>) -> Rc<Stack<Object>> {
    fn f<T, F>((comp, stack): (&Compiler, &Rc<Stack<T>>), dict: &Dictionary, name: &str, f: F)
    where
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
    f(s, dict, "@byte", move |s| {
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
    let int2 = int.clone();
    f(s, dict, "@refcount", move |s| {
        int2.push(s.pop()?.refs().len().into())
    });
    let int2 = int.clone();
    f(s, dict, "@bytecount", move |s| {
        int2.push(s.pop()?.data().len().into())
    });
    f(s, dict, "@concat", move |s| {
        let y = s.pop()?;
        let x = s.pop()?;
        s.push(x.concat(&y))
    });
    let int2 = int.clone();
    f(s, dict, "@slice", move |s| {
        let f = || Ok::<_, Box<dyn std::error::Error>>(usize::try_from(int2.pop()?)?);
        let f = || f().and_then(|end| Ok(f()?..end));
        f().and_then(|refs| s.push(s.pop()?.slice(f()?, refs)))
    });
    f(s, dict, "@intoref", move |s| {
        s.push(Object::from([s.pop()?]))
    });
    stack
}
