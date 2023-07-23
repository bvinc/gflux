use super::window::WindowComponent;
use crate::AppState;
use gflux::{Component, ComponentCtx, ComponentHandle};
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[allow(dead_code)]
pub struct AppComponent {
    app: gtk::Application,
    win_components: Rc<RefCell<Vec<ComponentHandle<WindowComponent>>>>,
}

impl Component for AppComponent {
    type GlobalModel = AppState;
    type Model = AppState;
    type Widget = gtk::Application;
    type Params = ();

    fn widget(&self) -> Self::Widget {
        self.app.clone()
    }

    fn build(ctx: ComponentCtx<Self>, _params: ()) -> Self {
        let app = gtk::Application::builder()
            .application_id("com.github.bvinc.gflux.todo")
            .build();

        let win_components = Rc::new(RefCell::new(vec![]));

        app.connect_activate(clone!(@strong win_components => move |app| {
            let c: ComponentHandle<WindowComponent> =
                ctx.create_child(|s: &mut AppState| s, app.clone());

            c.widget().present();

            win_components.borrow_mut().push(c);
        }));

        Self {
            app,
            win_components,
        }
    }

    fn rebuild(&mut self, _ctx: ComponentCtx<Self>) {}
}
