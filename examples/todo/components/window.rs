use super::list::ListComponent;
use crate::AppState;
use gflux::{Component, ComponentCtx, ComponentHandle};
use gtk::{prelude::*, Orientation};

#[allow(dead_code)]
pub struct WindowComponent {
    window: gtk::ApplicationWindow,
    list_comps: Vec<ComponentHandle<ListComponent>>,
}

impl Component for WindowComponent {
    type GlobalModel = AppState;
    type Model = AppState;
    type Widget = gtk::ApplicationWindow;
    type Params = gtk::Application;

    fn widget(&self) -> Self::Widget {
        self.window.clone()
    }

    fn build(ctx: ComponentCtx<Self>, app: gtk::Application) -> Self {
        // Create a window and set the title
        let window = gtk::ApplicationWindow::builder()
            .application(&app)
            .width_request(400)
            .height_request(500)
            .title("My Todo App")
            .build();

        let mut list_comps = Vec::new();

        let vbox = gtk::Box::new(Orientation::Vertical, 8);
        let c: ComponentHandle<ListComponent> =
            ctx.create_child(|s: &mut AppState| &mut s.tasks, ());
        vbox.append(&c.widget());

        window.set_child(Some(&vbox));
        list_comps.push(c);

        // Present window
        window.present();

        Self { window, list_comps }
    }

    fn rebuild(&mut self, _ctx: ComponentCtx<Self>) {}
}
