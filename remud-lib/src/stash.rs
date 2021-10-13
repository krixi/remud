#![allow(dead_code)]

extern crate proc_macro;

use std::{
    any::{type_name, Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use itertools::Itertools;
use remud_macros::all_tuples;
use thiserror::Error;

// Type requirements for values that can be stored in the Repo
pub trait Value: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Value for T {}

// Implementors can be queried against a Repo
pub trait StashQuery {
    type Fetch: for<'a> Fetch<'a>;
}

// Implementors can be retrieved from a Repo
pub trait Fetch<'w>: Sized {
    // The type this fetch will return
    type Item;

    // Creates the Fetch object
    fn init() -> Self;

    // Lists missing types from this and child fetches
    fn missing_types(&self, repo: &Stash) -> Vec<&'static str>;

    // Retrieves the data from the Repo
    fn fetch(&self, repo: &'w Stash) -> Self::Item;
}

// Blanket implementation for immutable queries of value types
impl<T: Value> StashQuery for &T {
    type Fetch = ReadFetch<T>;
}

// Blanket implementation of mutable queries of value types
impl<T: Value> StashQuery for &mut T {
    type Fetch = WriteFetch<T>;
}

// Type for an immutable fetch
pub struct ReadFetch<T> {
    marker: PhantomData<T>,
}

pub enum Immutable<'w, T> {
    Rc(Ref<'w, T>),
    Ref(&'w T),
}

impl<'w, T> Deref for Immutable<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Immutable::Rc(rc) => &*rc,
            Immutable::Ref(r) => *r,
        }
    }
}

pub enum Mutable<'w, T> {
    Rc(RefMut<'w, T>),
    Ref(&'w mut T),
}

impl<'w, T> Deref for Mutable<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Mutable::Rc(rc) => &*rc,
            Mutable::Ref(r) => *r,
        }
    }
}

impl<'w, T> DerefMut for Mutable<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Mutable::Rc(rc) => &mut *rc,
            Mutable::Ref(r) => *r,
        }
    }
}

impl<'w, T: Value> Fetch<'w> for ReadFetch<T> {
    type Item = Immutable<'w, T>;

    fn init() -> Self {
        ReadFetch {
            marker: PhantomData,
        }
    }

    fn missing_types(&self, stash: &Stash) -> Vec<&'static str> {
        let id = TypeId::of::<T>();
        if stash.data.contains_key(&id) {
            vec![]
        } else {
            vec![type_name::<T>()]
        }
    }

    fn fetch(&self, repo: &'w Stash) -> Self::Item {
        Immutable::Rc(repo.get::<T>().unwrap())
    }
}

// Type for a mutable fetch
pub struct WriteFetch<T> {
    marker: PhantomData<T>,
}

impl<'w, T: Value> Fetch<'w> for WriteFetch<T> {
    type Item = Mutable<'w, T>;

    fn init() -> Self {
        WriteFetch {
            marker: PhantomData,
        }
    }

    fn missing_types(&self, stash: &Stash) -> Vec<&'static str> {
        let id = TypeId::of::<T>();
        if stash.data.contains_key(&id) {
            vec![]
        } else {
            vec![type_name::<T>()]
        }
    }

    fn fetch(&self, stash: &'w Stash) -> Self::Item {
        Mutable::Rc(stash.get_mut::<T>().unwrap())
    }
}

// pub trait StashArgs {
//     type Collect: for<'w> Collect<'w, Self>;
// }

// // Implementors can be retrieved from a Repo
// pub trait Collect<'w, T>: Sized {
//     // The type this fetch will return
//     type Item;

//     // Creates the Fetch object
//     fn init(values: T) -> Self;

//     // Retrieves the data from the Repo
//     fn fetch(&self, id: TypeId) -> Self::Item;
// }

// // Blanket implementation for immutable queries of value types
// impl<T: Value> StashArgs for &T {
//     type Collect = ReadCollect<T>;
// }

// struct ReadCollect<'w, T> {
//     id: TypeId,
//     data: Immutable<'w, T>,
// }

// impl<'w, T> Collect<'w, T> for ReadCollect<'w, T> {
//     type Item = Immutable<'w, T>;

//     fn init(value: &T) -> Self {
//         ReadCollect {
//             id: TypeId::of::<T>(),
//             data: Immutable::Ref(value),
//         }
//     }

//     fn fetch(&self, id: TypeId) -> Self::Item {
//         todo!()
//     }
// }

// // Blanket implementation of mutable queries of value types
// impl<T: Value> StashArgs for &mut T {
//     type Collect = WriteCollect<T>;
// }

// struct WriteCollect<T> {}

// impl<'w, T> Collect<'w, T> for WriteCollect<T> {
//     type Item = Mutable<'w, T>;

//     fn init(values: &mut T) -> Self {
//         todo!()
//     }

//     fn fetch(&self, id: TypeId) -> Self::Item {
//         todo!()
//     }
// }

#[derive(Error, Debug)]
pub enum StashError {
    #[error("query does not match available data, missing types: {0}")]
    MissingValues(String),
}

// A collection which can hold at most one of each type.
// Allows dynamic queries of contents using `query`.
#[derive(Default)]
pub struct Stash {
    data: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Stash {
    // Inserts a value into the Repo
    pub fn insert<T: Value>(&mut self, value: T) {
        let id = TypeId::of::<T>();
        self.data.insert(id, RefCell::new(Box::new(value)));
    }

    // Retrieves an immutable reference to an value in the Repo, if it exists
    pub fn get<T: Value>(&self) -> Option<Ref<T>> {
        let id = TypeId::of::<T>();
        self.data
            .get(&id)
            .and_then(|v| v.try_borrow().ok())
            .map(|r| Ref::map(r, |v| v.downcast_ref::<T>().unwrap()))
    }

    // Retrieves a mutable reference to a value in the Repo, if it exists
    pub fn get_mut<T: Value>(&self) -> Option<RefMut<T>> {
        let id = TypeId::of::<T>();
        self.data
            .get(&id)
            .and_then(|v| v.try_borrow_mut().ok())
            .map(|r| RefMut::map(r, |v| v.downcast_mut::<T>().unwrap()))
    }

    // Queries the repo for a set of items, returning an error if one or more doesn't exist.
    pub fn query<Q: StashQuery>(&mut self) -> Result<<Q::Fetch as Fetch>::Item, StashError> {
        let fetch = <Q::Fetch as Fetch>::init();
        let missing = fetch.missing_types(self);
        if missing.is_empty() {
            Ok(fetch.fetch(self))
        } else {
            Err(StashError::MissingValues(missing.into_iter().join(", ")))
        }
    }

    // // Queries the repo for a set of items, returning an error if one or more doesn't exist.
    // pub fn query_with<Q: StashQuery>(
    //     &mut self,
    //     A: StashArgs,
    // ) -> Result<<Q::Fetch as Fetch>::Item, StashError> {
    //     let fetch = <Q::Fetch as Fetch>::init();
    //     let missing = fetch.missing_types(self);
    //     if missing.is_empty() {
    //         Ok(fetch.fetch(self))
    //     } else {
    //         Err(StashError::MissingValues(missing.into_iter().join(", ")))
    //     }
    // }
}

macro_rules! impl_tuple_fetch {
    ($($name: ident),*) => {
        #[allow(non_snake_case)]
        impl <'w, $($name: Fetch<'w>),*> Fetch<'w> for ($($name,)*) {
            type Item = ($($name::Item,)*);

            fn init() -> Self {
                ($($name::init(),)*)
            }

            fn missing_types(&self, _stash: &Stash) -> Vec<&'static str> {
                let ($($name,)*) = self;
                std::iter::empty()$(.chain($name.missing_types(_stash)))*.collect_vec()
            }

            fn fetch(&self, _stash: &'w Stash) -> Self::Item {
                let ($($name,)*) = self;
                ($($name.fetch(_stash),)*)
            }
        }

        impl<$($name: StashQuery),*> StashQuery for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
        }
    }
}

all_tuples!(impl_tuple_fetch, 1, 16, F);

#[cfg(test)]
mod test {
    use crate::stash::{Stash, StashError};

    #[test]
    fn test_stash_round_trip() {
        let mut stash = Stash::default();
        let value: usize = 5;

        stash.insert(value);

        assert_eq!(value, *stash.get::<usize>().unwrap());
    }

    #[test]
    fn test_stash_insert_overwrites() {
        let mut stash = Stash::default();
        let v1: usize = 2;
        stash.insert(v1);
        let v2: usize = 5;
        stash.insert(v2);

        assert_eq!(v2, *stash.get::<usize>().unwrap());
    }

    #[test]
    fn test_stash_get_missing_data() {
        let stash = Stash::default();

        assert!(stash.get::<usize>().is_none());
    }

    #[test]
    fn test_stash_query() {
        let mut stash = Stash::default();
        let v1: usize = 5;
        let v2: i32 = 3;
        stash.insert(v1);
        stash.insert(v2);
        let (qv1, qv2) = stash.query::<(&usize, &i32)>().unwrap();
        assert_eq!((v1, v2), (*qv1, *qv2));
    }

    #[test]
    fn test_stash_query_no_match() {
        let mut stash = Stash::default();
        if let Err(StashError::MissingValues(s)) = stash.query::<&usize>() {
            assert!(s.contains("usize"));
        } else {
            panic!("expected DoesNotMatch");
        };
    }

    #[test]
    fn test_stash_query_mut() {
        let mut stash = Stash::default();
        const NEW: usize = 10;

        let value: usize = 5;
        stash.insert(value);

        {
            let mut change = stash.query::<&mut usize>().unwrap();
            *change = NEW;
        }

        let qv1 = stash.query::<&usize>().unwrap();
        assert_eq!((NEW), (*qv1));
    }
}
