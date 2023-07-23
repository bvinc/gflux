//! gflux is a reactive component system designed to make GTK more manageable

#![allow(clippy::type_complexity)]
#![warn(rustdoc::all)]
#![warn(missing_debug_implementations)]

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::rc::{Rc, Weak};

/// The trait that defines a component
pub trait Component {
    /// The global application state
    type GlobalModel;
    /// The application state for this component
    type Model;
    /// The root widget type for this component
    type Widget;
    /// The parameters needed to build this component
    type Params;

    /// Returns the root widget
    fn widget(&self) -> Self::Widget;
    /// Builds the component
    fn build(ctx: ComponentCtx<Self>, params: Self::Params) -> Self;
    /// Runs after building and after model is mutated
    fn rebuild(&mut self, ctx: ComponentCtx<Self>);
}

/// Manages the component tree
#[derive(Clone)]
pub struct ComponentTree<M> {
    global: Rc<RefCell<M>>,
    comp_table: Rc<RefCell<ComponentTable>>,
    change_cbs: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
}

impl<M: fmt::Debug> fmt::Debug for ComponentTree<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentTree")
            .field("global", &self.global)
            .field("comp_table", &self.comp_table)
            .finish()
    }
}

impl<M> ComponentTree<M> {
    pub fn new(global: Rc<RefCell<M>>) -> Self {
        Self {
            global,
            comp_table: Rc::new(RefCell::new(ComponentTable::new())),
            change_cbs: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Register a callback that gets executed every time any component changes
    pub fn on_first_change<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        self.change_cbs.borrow_mut().push(Box::new(f));
    }

    /// Execute rebuild on every dirty component, and their ancestors, from the top down.
    pub fn exec_rebuilds(&self) {
        let mut all_dirty = BTreeSet::new();
        let mut new_dirty = BTreeSet::new();
        let mut dirty_parents = BTreeSet::new();
        for cid in &self.comp_table.borrow().dirty {
            new_dirty.insert(*cid);
        }
        while !new_dirty.is_empty() {
            for cid in &new_dirty {
                if let Some(parent_id) = self
                    .comp_table
                    .borrow()
                    .map
                    .get(cid)
                    .and_then(|c| c.upgrade())
                    .and_then(|c| c.borrow().parent_id())
                {
                    dirty_parents.insert(parent_id);
                }
            }
            all_dirty.append(&mut new_dirty);
            new_dirty.append(&mut dirty_parents);
        }
        for cid in &all_dirty {
            let weak_c = self
                .comp_table
                .borrow_mut()
                .map
                .get(cid)
                .and_then(|c| c.upgrade());
            if let Some(c) = weak_c {
                c.borrow_mut().rebuild();
            }
        }

        self.comp_table.borrow_mut().dirty.clear();
    }

    /// Create a new root component
    pub fn new_component<F, C>(&self, lens: F, params: C::Params) -> ComponentHandle<C>
    where
        C: Component<GlobalModel = M> + 'static,
        F: Fn(&mut C::GlobalModel) -> &mut C::Model + 'static,
    {
        let id = self.comp_table.borrow_mut().reserve_id();
        let mut ctx = ComponentCtx {
            global: self.global.clone(),
            comp_table: self.comp_table.clone(),
            change_cbs: self.change_cbs.clone(),
            id,
            parent_id: None,
            lens: Rc::new(lens),
        };

        let mut component = C::build(ctx.clone(), params);
        component.rebuild(ctx.clone());
        let c = Rc::new(RefCell::new(ComponentBase {
            ctx: ctx.clone(),
            component,
        }));

        ctx.id = ctx
            .comp_table
            .borrow_mut()
            .insert(id, Rc::downgrade(&c) as WeakComponentBase);

        ComponentHandle { inner: c }
    }
}

#[derive(Debug)]
struct ComponentTable {
    pub next_id: ComponentId,
    pub map: HashMap<ComponentId, WeakComponentBase>,
    pub dirty: HashSet<ComponentId>,
}

impl ComponentTable {
    fn new() -> Self {
        Self {
            next_id: 1,
            map: HashMap::new(),
            dirty: HashSet::new(),
        }
    }

    fn reserve_id(&mut self) -> ComponentId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn insert(&mut self, cid: ComponentId, c: WeakComponentBase) -> ComponentId {
        self.map.insert(cid, c);
        cid
    }

    fn is_clean(&self) -> bool {
        self.dirty.is_empty()
    }

    fn mark_dirty(&mut self, cid: ComponentId) {
        self.dirty.insert(cid);
    }

    fn destroy(&mut self, cid: ComponentId) {
        self.map.remove(&cid);
        self.dirty.remove(&cid);
    }
}

/// Handle for a component
#[derive(Debug)]
pub struct ComponentHandle<C: Component> {
    inner: Rc<RefCell<ComponentBase<C>>>,
}

impl<C: Component> ComponentHandle<C> {
    /// Returns the root widget
    pub fn widget(&self) -> C::Widget {
        self.inner.borrow().component.widget()
    }

    /// Rebuilds the component.  You shouldn't need to call this manually if
    /// you've mutated this component's state using its `ComponentCtx`.
    pub fn rebuild(&self) {
        self.inner.borrow_mut().rebuild()
    }
}

#[derive(Debug)]
struct ComponentBase<C: Component> {
    ctx: ComponentCtx<C>,
    component: C,
}

impl<C: Component> ComponentBaseTrait for ComponentBase<C> {
    fn id(&self) -> ComponentId {
        self.ctx.id
    }
    fn parent_id(&self) -> Option<ComponentId> {
        self.ctx.parent_id
    }
    fn rebuild(&mut self) {
        self.component.rebuild(self.ctx.clone());
    }
}

impl<C: Component> Drop for ComponentBase<C> {
    fn drop(&mut self) {
        self.ctx.comp_table.borrow_mut().destroy(self.ctx.id);
    }
}

trait ComponentBaseTrait {
    fn id(&self) -> ComponentId;
    fn parent_id(&self) -> Option<ComponentId>;
    fn rebuild(&mut self);
}

type ComponentId = u64;
type WeakComponentBase = Weak<RefCell<dyn ComponentBaseTrait>>;

/// Performs bookkeeping for the component, and provides state accessor methods
pub struct ComponentCtx<C: Component + ?Sized> {
    id: ComponentId,
    parent_id: Option<ComponentId>,

    global: Rc<RefCell<C::GlobalModel>>,
    comp_table: Rc<RefCell<ComponentTable>>,
    change_cbs: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
    lens: Rc<dyn Fn(&mut C::GlobalModel) -> &mut C::Model>,
}

impl<C: Component + fmt::Debug> fmt::Debug for ComponentCtx<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentTree")
            .field("id", &self.id)
            .field("parent_id", &self.parent_id)
            .field("comp_table", &self.comp_table)
            .finish()
    }
}

impl<C: Component + ?Sized> Clone for ComponentCtx<C> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            parent_id: self.parent_id,
            global: self.global.clone(),
            comp_table: self.comp_table.clone(),
            change_cbs: self.change_cbs.clone(),
            lens: self.lens.clone(),
        }
    }
}

impl<C: Component> ComponentCtx<C> {
    /// Creates a component that is a child of this component
    pub fn create_child<K: Component<GlobalModel = C::GlobalModel> + 'static, F>(
        &self,
        p_to_c: F,
        params: K::Params,
    ) -> ComponentHandle<K>
    where
        F: Fn(&mut C::Model) -> &mut K::Model + 'static,
        C::Model: 'static,
        C::GlobalModel: 'static,
        K::Model: 'static,
        K::GlobalModel: 'static,
    {
        let p_lens = self.lens.clone();
        let child_lens: Rc<dyn Fn(&mut C::GlobalModel) -> &mut K::Model> =
            Rc::new(move |g| p_to_c(p_lens(g)));

        let id = self.comp_table.borrow_mut().reserve_id();
        let mut ctx = ComponentCtx {
            id,
            parent_id: Some(self.id),
            comp_table: self.comp_table.clone(),
            change_cbs: self.change_cbs.clone(),
            global: self.global.clone(),
            lens: child_lens,
        };
        let mut component = K::build(ctx.clone(), params);
        component.rebuild(ctx.clone());
        let c = Rc::new(RefCell::new(ComponentBase {
            ctx: ctx.clone(),
            component,
        }));

        ctx.id = ctx
            .comp_table
            .borrow_mut()
            .insert(id, Rc::downgrade(&c) as WeakComponentBase);

        ComponentHandle { inner: c }
    }

    /// Access the component state
    pub fn with_model<R, F: Fn(&C::Model) -> R>(&self, f: F) -> R {
        let mut global = self.global.borrow_mut();
        let lens = self.lens.clone();
        f(lens(&mut global))
    }

    /// Access the component state mutably, marks the component as dirty.
    pub fn with_model_mut<R, F: Fn(&mut C::Model) -> R>(&self, f: F) -> R {
        let was_clean = self.comp_table.borrow().is_clean();
        self.comp_table.borrow_mut().mark_dirty(self.id);

        let mut global = self.global.borrow_mut();
        let lens = self.lens.clone();
        let r = f(lens(&mut global));

        let change_cbs = self.change_cbs.borrow();
        if was_clean {
            for cb in &*change_cbs {
                cb()
            }
        }
        r
    }
}
