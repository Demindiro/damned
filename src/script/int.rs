use super::{Compiler, DictionaryData, Stack};
use num::BigInt;
use std::rc::Rc;

pub fn define(comp: &Compiler, dict: &mut DictionaryData) -> Rc<Stack<BigInt>> {
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
    let stack = Rc::new(Stack::<BigInt>::default());
    let s = (comp, &stack);
    f(s, dict, "+", |s| s.op2to1(|x, y| x + y));
    f(s, dict, "-", |s| s.op2to1(|x, y| x - y));
    f(s, dict, "*", |s| s.op2to1(|x, y| x * y));
    f(s, dict, "=", |s| s.op2to1(|x, y| (x == y).into()));
    f(s, dict, "<>", |s| s.op2to1(|x, y| (x != y).into()));
    f(s, dict, "<", |s| s.op2to1(|x, y| (x < y).into()));
    f(s, dict, ">", |s| s.op2to1(|x, y| (x > y).into()));
    f(s, dict, "<=", |s| s.op2to1(|x, y| (x <= y).into()));
    f(s, dict, ">=", |s| s.op2to1(|x, y| (x >= y).into()));
    f(s, dict, "#dup", |s| {
        let x = s.pop()?;
        s.push(x.clone())?;
        s.push(x)
    });
    f(s, dict, "#drop", |s| s.pop().map(|_| ()));
    f(s, dict, "#swap", |s| {
        let x = s.pop()?;
        let y = s.pop()?;
        s.push(x)?;
        s.push(y)
    });
    let s = stack.clone();
    let comp = comp.clone();
    dict.push_alt(move |name| {
        let f = |x: BigInt| {
            let s = s.clone();
            comp.with(move || s.push(x.clone()))
        };
        if name.len() > 2 && name.starts_with("'") && name.ends_with("'") {
            let mut it = name.chars().skip(1);
            let c = match it.next().unwrap() {
                '\\' => match it.next().unwrap() {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    c => todo!("{c:?}"),
                },
                c => c,
            };
            assert_eq!(it.next().unwrap(), '\'');
            return Some(f(BigInt::from(c as u32)));
        }
        name.parse::<BigInt>().ok().map(f)
    });
    stack
}
