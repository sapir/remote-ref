//! This library allows sharing references to objects across thread boundaries,
//! even when those objects aren't `Send` or `Sync`. The objects themselves are
//! held in an `ObjectStore` struct that isn't necessarily `Send`/`Sync`, and so
//! the objects can still only be actually used on the owning thread.
//!
//! This differs from some other crates such as
//! [`fragile`](https://crates.io/crates/fragile) or
//! [`send_wrapper`](https://crates.io/crates/send_wrapper) in that the access
//! rule is enforced at compile time, and that the `ObjectStore` (currently)
//! requires an extra garbage collection function to be called manually.

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
    rc: Arc<PhantomInvariantAlwaysSendSync<T>>,
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
/// objects, or else [`ObjectStore::remove`] should be called on objects when
/// dropping them.
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
            rc: rc_for_return,
        }
    }

    /// Remove an object reference from the object store. If the reference count
    /// is then zero, the stored object is dropped and returned. If there are
    /// still any other active references, None is returned.
    ///
    /// # Panics
    ///
    /// Panics if the reference doesn't belong to this store.
    pub fn remove(&mut self, obj_ref: ObjectRef<T>) -> Option<T> {
        let index = obj_ref.index;

        // Verify that we're using the correct store
        assert_eq!(Arc::as_ptr(&obj_ref.rc), Weak::as_ptr(&self.slab[index].rc));

        if Arc::try_unwrap(obj_ref.rc).is_ok() {
            // That was the last strong reference - remove the object from the
            // store.
            Some(self.slab.remove(index).data)
        } else {
            None
        }
    }
}
