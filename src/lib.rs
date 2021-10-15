//! A crate to allow sharing references to objects across thread boundaries,
//! even when those objects aren't `Send` or `Sync`. The objects themselves are
//! held in an [`ObjectStore`], and can still only be actually used on the
//! owning thread.

use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use rich_phantoms::PhantomInvariantAlwaysSendSync;
use slab::Slab;

/// A reference to an object in an [`ObjectStore`]. This can be held in any
/// thread, even if `T` isn't `Send` or `Sync`, because in such a case, to
/// access the object, you'll still need to be on the thread owning the
/// `ObjectStore`.
#[must_use]
#[derive(Clone, Debug)]
pub struct ObjectRef<T> {
    index: usize,
    _rc: Arc<PhantomInvariantAlwaysSendSync<T>>,
}

struct Object<T> {
    rc: Weak<PhantomInvariantAlwaysSendSync<T>>,
    data: T,
}

/// A storage allowing references to objects that aren't `Send` or `Sync`. The
/// references ([`ObjectRef`]s) can be held in other threads, even if `T` isn't
/// `Send` or `Sync`, because in such a case, to access the object, you'll still
/// need to be on the thread owning the [`ObjectStore`].
///
/// `ObjectStore::clean` should be called once in a while to drop any unused
/// objects.
pub struct ObjectStore<T> {
    slab: Slab<Object<T>>,
}

impl<T> Default for ObjectStore<T> {
    fn default() -> Self {
        Self { slab: Slab::new() }
    }
}

impl<T> ObjectStore<T> {
    pub fn get(&self, obj_ref: &ObjectRef<T>) -> &T {
        &self.slab[obj_ref.index].data
    }

    pub fn get_mut(&mut self, obj_ref: &ObjectRef<T>) -> &mut T {
        &mut self.slab[obj_ref.index].data
    }

    /// Garbage-collects unused objects.
    pub fn clean(&mut self) {
        // Note that `slab.retain` makes sure that indexes all stay valid even
        // when elements are removed, unlike `Vec::retain`.
        self.slab.retain(|_i, obj| obj.rc.strong_count() > 0)
    }

    pub fn insert(&mut self, data: T) -> ObjectRef<T> {
        let rc = Arc::new(PhantomData);
        let rc_for_return = rc.clone();

        let obj = Object {
            rc: Arc::downgrade(&rc),
            data,
        };

        let index = self.slab.insert(obj);

        ObjectRef {
            index,
            _rc: rc_for_return,
        }
    }
}
