use super::{Compiler, Dictionary, Stack};
use num::BigInt;
use std::rc::Rc;

pub fn define(comp: &Compiler, dict: &Dictionary, stack: &Rc<Stack<BigInt>>) {
    fn f<T, F>((comp, stack, dict): (&Compiler, &Rc<Stack<T>>, &Dictionary), name: &str, f: F)
    where
        F: 'static + Fn(&Stack<T>) -> super::Result<()> + 'static,
        // TODO why 'static?
        T: 'static,
    {
        let stack = stack.clone();
        dict.define(name, comp.with(move || (f)(&stack)));
    }
    let s = (comp, stack, dict);
    f(s, "+", |s| s.op2to1(|x, y| x + y));
    f(s, "-", |s| s.op2to1(|x, y| x - y));
    f(s, "*", |s| s.op2to1(|x, y| x * y));
    f(s, "=", |s| s.op2to1(|x, y| (x == y).into()));
    f(s, "<>", |s| s.op2to1(|x, y| (x != y).into()));
    f(s, "<", |s| s.op2to1(|x, y| (x < y).into()));
    f(s, ">", |s| s.op2to1(|x, y| (x > y).into()));
    f(s, "<=", |s| s.op2to1(|x, y| (x <= y).into()));
    f(s, ">=", |s| s.op2to1(|x, y| (x >= y).into()));
    f(s, "#dup", |s| {
        let x = s.pop()?;
        s.push(x.clone())?;
        s.push(x)
    });
    f(s, "#drop", |s| s.pop().map(|_| ()));
    f(s, "#swap", |s| {
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
}
