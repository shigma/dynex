use dyn_traits::*;

use std::fmt::Debug;

#[macro_use]
extern crate dyn_derive;

#[dyn_impl]
pub trait Meta: Debug + PartialEq + Clone + Add {}

impl PartialEq for dyn Meta {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_any())
    }
}

// Sized
impl Clone for Box<dyn Meta> {
    fn clone(&self) -> Self {
        ptr::convert_to_box(self, DynClone::dyn_clone)
    }
}

impl std::ops::Add for Box<dyn Meta> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        dyn_traits::ptr::convert_into_box(self, |m| m.dyn_add(other.as_any_box()))
    }
}

#[test]
fn derive() {

    #[derive(Debug, PartialEq, Clone)]
    pub struct MetaImpl {
        name: String,
    }

    impl Meta for MetaImpl {}
    
    impl std::ops::Add for MetaImpl {
        type Output = MetaImpl;
    
        fn add(self, other: MetaImpl) -> MetaImpl {
            MetaImpl {
                name: format!("{}{}", self.name, other.name),
            }
        }
    }


    #[derive(Debug, Clone, PartialEqFix)]
    pub struct Bar {
        meta: Box<dyn Meta>,
    }

    impl std::ops::Add for Bar {
        type Output = Bar;
    
        fn add(self, other: Bar) -> Bar {
            Bar {
                meta: self.meta + other.meta,
            }
        }
    }


    let bar1 = Bar { meta: Box::new(MetaImpl { name: "foo".into() }) };
    let bar2 = Bar { meta: Box::new(MetaImpl { name: "bar".into() }) };
    assert_eq!(bar1, Bar { meta: Box::new(MetaImpl { name: "foo".into() }) });
    let bar3 = bar1 + bar2;
    println!("{:?}", bar3);

    // assert_eq!(bar3.foo.magic(), 42);
    panic!();
}
