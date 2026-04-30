use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use soul_ecs_sys as sys;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct BorrowKey {
    entity: sys::ecs_entity_t,
    component: sys::ecs_id_t,
}

enum BorrowState {
    Shared(usize),
    Mutable,
}

#[derive(Default)]
pub(crate) struct BorrowTracker {
    borrows: HashMap<BorrowKey, BorrowState>,
}

impl BorrowTracker {
    fn acquire_shared(&mut self, key: BorrowKey) {
        match self.borrows.get_mut(&key) {
            Some(BorrowState::Shared(count)) => {
                *count += 1;
            }
            Some(BorrowState::Mutable) => {
                panic!("component is already mutably borrowed");
            }
            None => {
                self.borrows.insert(key, BorrowState::Shared(1));
            }
        }
    }

    fn acquire_mutable(&mut self, key: BorrowKey) {
        match self.borrows.get(&key) {
            Some(BorrowState::Shared(_)) => {
                panic!("component is already shared borrowed");
            }
            Some(BorrowState::Mutable) => {
                panic!("component is already mutably borrowed");
            }
            None => {
                self.borrows.insert(key, BorrowState::Mutable);
            }
        }
    }

    fn release_shared(&mut self, key: BorrowKey) {
        match self.borrows.get_mut(&key) {
            Some(BorrowState::Shared(1)) => {
                self.borrows.remove(&key);
            }
            Some(BorrowState::Shared(count)) => {
                *count -= 1;
            }
            Some(BorrowState::Mutable) | None => {
                unreachable!("shared component borrow state is inconsistent");
            }
        }
    }

    fn release_mutable(&mut self, key: BorrowKey) {
        match self.borrows.remove(&key) {
            Some(BorrowState::Mutable) => {}
            Some(BorrowState::Shared(_)) | None => {
                unreachable!("mutable component borrow state is inconsistent");
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct BorrowContext {
    tracker: Rc<RefCell<BorrowTracker>>,
    world: *mut sys::ecs_world_t,
}

impl BorrowContext {
    pub(crate) fn new(tracker: Rc<RefCell<BorrowTracker>>, world: *mut sys::ecs_world_t) -> Self {
        Self { tracker, world }
    }

    pub(crate) fn shared(
        &self,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
    ) -> ComponentBorrowGuard {
        ComponentBorrowGuard::shared(Rc::clone(&self.tracker), entity, component)
    }

    pub(crate) fn mutable(
        &self,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
        notify_modified: bool,
    ) -> ComponentBorrowGuard {
        ComponentBorrowGuard::mutable(
            Rc::clone(&self.tracker),
            self.world,
            entity,
            component,
            notify_modified,
        )
    }
}

pub(crate) struct ComponentBorrowGuard {
    tracker: Rc<RefCell<BorrowTracker>>,
    key: BorrowKey,
    kind: ComponentBorrowKind,
}

enum ComponentBorrowKind {
    Shared,
    Mutable {
        world: *mut sys::ecs_world_t,
        notify_modified: bool,
    },
}

impl ComponentBorrowGuard {
    pub(crate) fn shared(
        tracker: Rc<RefCell<BorrowTracker>>,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
    ) -> Self {
        let key = BorrowKey { entity, component };
        tracker.borrow_mut().acquire_shared(key);
        Self {
            tracker,
            key,
            kind: ComponentBorrowKind::Shared,
        }
    }

    pub(crate) fn mutable(
        tracker: Rc<RefCell<BorrowTracker>>,
        world: *mut sys::ecs_world_t,
        entity: sys::ecs_entity_t,
        component: sys::ecs_id_t,
        notify_modified: bool,
    ) -> Self {
        let key = BorrowKey { entity, component };
        tracker.borrow_mut().acquire_mutable(key);
        Self {
            tracker,
            key,
            kind: ComponentBorrowKind::Mutable {
                world,
                notify_modified,
            },
        }
    }
}

impl Drop for ComponentBorrowGuard {
    fn drop(&mut self) {
        match self.kind {
            ComponentBorrowKind::Shared => {
                self.tracker.borrow_mut().release_shared(self.key);
            }
            ComponentBorrowKind::Mutable {
                world,
                notify_modified,
            } => {
                if notify_modified {
                    // SAFETY: the guard is only created for a live entity/component pair in this world.
                    unsafe { sys::ecs_modified_id(world, self.key.entity, self.key.component) };
                }
                self.tracker.borrow_mut().release_mutable(self.key);
            }
        }
    }
}
