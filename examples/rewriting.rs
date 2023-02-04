#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use smart_pat_bindings::smart_pat_bindings;

enum A {
    Opt1 {
        b: B,
        c: String,
    },
    Opt2(i32),
}

enum B {
    Opt1 {
        e: String,
    },
    Opt2 {
        f: i32,
    },
}

impl A {
    fn some_fn(&mut self) {
        #[smart_pat_bindings]
        match self {
            //  Doesn't work normally
            a@A::Opt1 { b: b@B::Opt1 { e }, .. } => {
                if e == "hello" {
                    *a = A::Opt2(10);
                } else {
                    *e = String::from("world");
                }
            }
            // Gets expanded into
            /*
            a @ A::Opt1 {
                b: B::Opt1 { e: _ },
                ..
            } => {
                let A :: Opt1 { b : b @ B :: Opt1 { e : _ } , .. } = a else { unsafe { :: core :: hint :: unreachable_unchecked () } } ;
                let B :: Opt1 { e } = b else { unsafe { :: core :: hint :: unreachable_unchecked () } } ;
                {
                    if e == "hello" {
                        *a = A::Opt2(10);
                    } else {
                        *e = String::from("world");
                    }
                }
            }
            */
            _ => (),
        }
    }
}

fn main() {
}
