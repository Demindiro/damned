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
    f(s, "#2dup", |s| {
        let y = s.pop()?;
        let x = s.pop()?;
        s.push(x.clone())?;
        s.push(y.clone())?;
        s.push(x)?;
        s.push(y)
    });
    f(s, "#drop", |s| s.pop().map(|_| ()));
    f(s, "#swap", |s| {
        let x = s.pop()?;
        let y = s.pop()?;
        s.push(x)?;
        s.push(y)
    });
    f(s, "#min", |s| s.op2to1(|x, y| x.min(y)));
    f(s, "#max", |s| s.op2to1(|x, y| x.max(y)));
    let to_usize = |x| usize::try_from(x).unwrap();
    f(s, "#bit:shl", move |s| s.op2to1(|x, y| x << to_usize(y)));
    f(s, "#bit:shr", move |s| s.op2to1(|x, y| x >> to_usize(y)));
    f(s, "#bit:and", move |s| s.op2to1(|x, y| x & y));
    f(s, "#bit:or", move |s| s.op2to1(|x, y| x | y));
    f(s, "#bit:xor", move |s| s.op2to1(|x, y| x ^ y));
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
        from_string(name).map(f)
    });
}

fn from_string(s: &str) -> Option<BigInt> {
    let (radix, s) = match s.get(..2) {
        Some("0b") => (2, &s[2..]),
        Some("0o") => (8, &s[2..]),
        Some("0x") => (16, &s[2..]),
        _ => (10, s),
    };

    let s = s.as_bytes();
    let (neg, s) = match s[0] {
        b'+' => (false, &s[1..]),
        b'-' => (true, &s[1..]),
        _ => (false, s),
    };

    s.iter()
        .try_fold(BigInt::ZERO, |n, c| {
            let x = match c {
                b'_' => return Some(n),
                b'0'..=b'9' => c - b'0',
                b'a'..=b'z' => c - b'a' + 10,
                b'A'..=b'Z' => c - b'A' + 10,
                _ => return None,
            };
            (x < radix).then(|| n * radix + x)
        })
        .map(|n| if neg { -n } else { n })
}
