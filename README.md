
#  gflux

gflux is a tiny experimental reactive component system for rust, designed to make GTK more manageable.

gflux:
* is about 300 lines of code
* contains no macros
* is independent of any particular GUI library
* tracks which components have had their model's mutated
* is orthogonal to any model diffing code needed to optimize updates

##  Why GTK is hard in rust

Let's look at a GTK Button.  You register a callback to handle a button click with this method

```rust
fn connect_clicked<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId
```

The `'static` lifetime bound means that the callback function provided can only capture static references.  This makes sense, because a GTK Button is a reference counted object that might live beyond the current stack frame.  But I believe this is the single greatest source of difficulty of GTK in rust.

There are usually 3 ways to handle this:
* Wrap your application state in `Rc<RefCell<T>>`.   This works when your application state is simple.  For complex applications, you don't want every widget to be aware of your entire application state.  This means you end up putting `Rc<RefCell<T>>` all over your application state.  This gets unmanageable.
* Send a message to a queue.  Many rust gtk component frameworks that currently exist, choose this method.  This works.  But occasionally, you want to provide a callback that blocks an action based on its return value.  For example, a delete-event on a window in GTK can return true or false.  Sending a message requires picking a return value without access to your application state.
* Create custom widgets and put your application state inside of them.  This is the GTK way.  This works, if you don't mind putting your application state inside of object oriented widgets and not maintaining separation.

## gflux components

gflux works by building a component tree.  Each component, on creation, is given a "lens" function.  A chain of lens functions from each component, works together to always be able to go from the global application state down to the state that an individual component cares about.

When a component is created, a lens function in provided.  But first, let's look at an example of a simple component for a task in a todo list.

```rust
use crate::{AppState, Task};
use glib::clone;
use gtk::{prelude::*, Align};
use gflux::{Component, ComponentCtx};

pub struct TaskComponent {
    hbox: gtk::Box,
    label: gtk::Label,
}
  
impl Component for TaskComponent {
    type GlobalModel = AppState;
    type Model = Task;
    type Widget = gtk::Box;
    type Params = ();

    // The root widget
    fn widget(&self) -> Self::Widget {
        self.hbox.clone()
    }

    // Called when the component is constructed
    fn build(ctx: ComponentCtx<Self>, params: ()) -> Self {
        let checkbox = gtk::CheckButton::new();
        checkbox.connect_toggled(clone!(@strong ctx => move  |cb| {
            ctx.with_model_mut(|task| task.done = cb.is_active());
        }));
  
        let label = gtk::Label::new(None);

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        hbox.append(&checkbox);
        hbox.append(&label);
  
        // rebuild will be called immediately afterwards
        Self { hbox, label }
    }

    // Called after a change in state is detected
    fn rebuild(&mut self, ctx: ComponentCtx<Self>) {
        let name = ctx.with_model(|task| task.name.clone());
        if ctx.with_model(|task| task.done) {
            // If the task is done, make it strikethrough
            let markup = format!("<s>{}</s>", glib::markup_escape_text(&name));
            self.label.set_markup(&markup);
        } else {
            self.label.set_text(&name);
        }
    }
}
```

When `rebuild` is called, it's up to the component to make sure the widgets match the `Task` struct in the component's model.

`ComponentCtx<Self>` does all of the component bookkeeping for you.  And most importantly, it provides access to the component's model, which is a simple `Task` struct.  It provides two methods for this: `with_model` and `with_model_mut`.

`with_model_mut` marks the component a dirty so that rebuild will be called soon afterwards.

### Initializing a component tree

```rust
// Create the global application state
let global = Rc::new(RefCell::new(AppState { ... }));

// Create the root of the component tree
let mut ctree = ComponentTree::new(global);
```

### Creating a component

Given a "lens" function, and parameters, you can create new component.  A similar method exists on a ComponentCtx to create a child component of an existing component.

```rust
let task_comp: ComponentHandle<TaskComponent> = ctree.new_component(|app_state| app_state.get_task_mut(), ());
```

### Change tracking

A component tree provides two methods:

* `on_first_change` to register a callback that gets called every time the component tree moves from a completely clean state to a dirty state.
* `rebuild_changed` to call the `rebuild` method on all components that have been marked as dirty by `with_model_mut`, and all of their ancestor components, from the oldest to the newest.

With both of these methods, we can register the GTK main loop to always rebuild any component whose model has been mutated:

```rust
// When the tree first moves from clean to dirty, use `idle_add_local_once`
// to make sure that `ctree.rebuild_changed()` later gets called from the gtk
// main loop
ctree.on_first_change(clone!(@strong ctree => move || {
    glib::source::idle_add_local_once(clone!(@strong ctree => move || ctree.rebuild_changed()));
}));
```

### Guidelines to having a good time

* Creating a component returns a ComponentHandle.  Keep these objects alive if your component still exists.
* Calls to `with_model` and `with_model_mut` should be kept short, and no GTK functions should be called from inside of them.  Copy out parts of the model before calling GTK functions.
* Avoid calling GTK functions that recursively call the main loop, such as `dialog.run()`

## Inspirations

* [https://github.com/antoyo/relm](Relm)
* Raph Levien's talk on [https://www.youtube.com/watch?v=XjbVnwBtVEk](Xilem)