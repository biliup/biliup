use indexmap::IndexMap;
use std::any::TypeId;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tracing::error;

pub type AnyMap<T> = HashMap<TypeId, T, BuildHasherDefault<IdHasher>>;

// With TypeIds as keys, there's no need to hash them. They are already hashes
// themselves, coming from the compiler. The IdHasher just holds the u64 of
// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Default)]
pub struct IdHasher(u64);

impl Hasher for IdHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }
}

#[derive(Clone)]
pub struct Cycle<T> {
    map: Arc<RwLock<IndexMap<String, T>>>,
}

impl<T: Copy> Cycle<T> {
    pub fn new(map: IndexMap<String, T>) -> Self {
        if map.is_empty() {
            unreachable!("list must not be empty")
        }
        Self {
            map: Arc::new(RwLock::new(map)),
        }
    }

    pub fn get(&self, i: &mut usize) -> (String, T) {
        match self.map.read().unwrap().get_index(*i) {
            Some((k, v)) => {
                *i += 1;
                (k.clone(), *v)
            }
            None => {
                *i = 0;
                self.get(i)
            }
        }
    }

    pub fn get_all(&self) -> IndexMap<String, T> {
        let read_guard = self.map.read().unwrap();
        read_guard.clone()
    }

    pub fn replace(&mut self, map: IndexMap<String, T>) {
        if map.is_empty() {
            unreachable!("list must not be empty")
        }
        *self.map.write().unwrap() = map;
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, IndexMap<String, T>> {
        self.map.write().unwrap()
    }

    pub fn change(&self, key: &str, value: T) {
        self.write()
            .entry(String::from(key))
            .and_modify(|mt| *mt = value);
    }

    pub fn insert(&self, key: String, value: T) {
        self.map.write().unwrap().insert(key, value);
    }
}

impl<T: Debug> Debug for Cycle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut temp = Vec::new();
        for (k, v) in self.map.read().unwrap().iter() {
            temp.push(format!(" {{ {}: {:?} }}", k, v));
        }
        write!(f, "{}", temp.join(","))?;
        write!(f, " }}")
    }
}

pub fn logging_spawn<T, O: 'static>(future: T) -> JoinHandle<T::Output>
where
    T: Future<Output = Result<O, Box<dyn Error + Send + Sync>>> + Send + 'static,
    T::Output: Send + 'static,
{
    tokio::spawn(async move {
        future.await.map_err(|e| {
            error!("spawn error: {}", e);
            e
        })
    })
}
